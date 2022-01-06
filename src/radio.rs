use log::{debug, info, trace};

use linux_embedded_hal::gpio_cdev::{Chip, EventRequestFlags, LineEventHandle, LineRequestFlags};
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::Delay;
use linux_embedded_hal::Spidev;

use embedded_hal::blocking::delay::DelayMs;
use rfm69::registers::Registers;
use rfm69::NoCs;
use rfm69::Rfm69;

use crate::sparsnasmetrics::SparsnasMetrics;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::unix::AsyncFd;

use sparsnasdecode::{SparsnasDecodeError, SparsnasDecoder};

use rfm69::registers::{
    DataMode, DccCutoff, DioMapping, DioMode, DioPin, DioType, InterPacketRxDelay, Modulation,
    ModulationShaping, ModulationType, PacketConfig, PacketDc, PacketFiltering, PacketFormat, RxBw,
    RxBwFsk,
};

use rfm69::SpiTransactional;

#[derive(Debug)]
pub enum SparsnasRadioError {
    RFM69,
    Pin(linux_embedded_hal::gpio_cdev::Error),
    IO(std::io::Error),
    Conf,
    NoContactWithChip,
}

impl<Ecs, Espi> From<rfm69::Error<Ecs, Espi>> for SparsnasRadioError {
    fn from(_: rfm69::Error<Ecs, Espi>) -> Self {
        SparsnasRadioError::RFM69
    }
}

impl From<linux_embedded_hal::gpio_cdev::Error> for SparsnasRadioError {
    fn from(e: linux_embedded_hal::gpio_cdev::Error) -> Self {
        SparsnasRadioError::Pin(e)
    }
}

impl From<std::io::Error> for SparsnasRadioError {
    fn from(e: std::io::Error) -> Self {
        SparsnasRadioError::IO(e)
    }
}

pub struct SparsnasRadio {
    rfm: Rfm69<NoCs, SpiTransactional<linux_embedded_hal::Spidev>, linux_embedded_hal::Delay>,
    irqevents: LineEventHandle,
    serial: u32,
    pulses_per_kwh: u32,
}

pub struct SparsnasRadioConfig {
    gpiochip: String,
    spidev: String,
    interrupt_pin: Option<u32>,
    reset_pin: Option<u32>,
    pulses_per_kwh: u32,
    serial: Option<u32>,
}

impl SparsnasRadioConfig {
    pub fn new() -> Self {
        SparsnasRadioConfig {
            gpiochip: "/dev/gpiochip0".to_string(),
            spidev: "/dev/spidev0.0".to_string(),
            interrupt_pin: None,
            reset_pin: None,
            pulses_per_kwh: 1000,
            serial: None,
        }
    }

    pub fn gpiochip_device(mut self, path: &str) -> Self {
        self.gpiochip = path.to_string();
        self
    }
    pub fn interrupt_pin(mut self, pin: u32) -> Self {
        self.interrupt_pin = Some(pin);
        self
    }
    pub fn reset_pin(mut self, pin: u32) -> Self {
        self.reset_pin = Some(pin);
        self
    }
    pub fn pulses_per_kwh(mut self, pulses: u32) -> Self {
        self.pulses_per_kwh = pulses;
        self
    }
    pub fn serial(mut self, serial: u32) -> Self {
        self.serial = Some(serial);
        self
    }

    pub fn build(&self) -> Result<SparsnasRadio, SparsnasRadioError> {
        SparsnasRadio::new(self)
    }
}

impl SparsnasRadio {
    pub fn new(conf: &SparsnasRadioConfig) -> Result<Self, SparsnasRadioError> {
        let serial = conf.serial.ok_or(SparsnasRadioError::Conf)?;
        let mut gpio = Chip::new(&conf.gpiochip)?;

        let mut spi = Spidev::open(&conf.spidev)?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(1_000_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&options)?;

        let mut rfm = Rfm69::new_without_cs(spi, Delay {});

        if let Some(pin) = conf.reset_pin {
            let resetpin = gpio
                .get_line(pin)?
                .request(LineRequestFlags::OUTPUT, 0, "reset-pin")?;

            trace!("Resetting RFM chip...");

            resetpin.set_value(1)?;

            Delay {}.delay_ms(300u32);

            resetpin.set_value(0)?;

            Delay {}.delay_ms(50u32);
        }

        let mut tries = 0;
        while rfm.read(Registers::SyncValue1)? != 0xAA {
            tries += 1;
            if tries > 10 {
                return Err(SparsnasRadioError::NoContactWithChip);
            }
            rfm.write(Registers::SyncValue1, 0xAA)?;
            Delay {}.delay_ms(100u32);
        }

        rfm.modulation(Modulation {
            data_mode: DataMode::Packet,
            modulation_type: ModulationType::Fsk,
            shaping: ModulationShaping::Shaping01,
        })?;

        rfm.bit_rate(40_000.0)?;

        rfm.frequency(867_987_500.0)?;

        rfm.fdev(10_000.0)?;

        rfm.rx_bw(RxBw {
            dcc_cutoff: DccCutoff::Percent4,
            rx_bw: RxBwFsk::Khz62dot5,
        })?;

        rfm.sync(&[0xd2, 0x01])?;

        rfm.rssi_threshold(0xbe)?;

        rfm.dio_mapping(DioMapping {
            pin: DioPin::Dio0,
            dio_type: DioType::Dio01,
            dio_mode: DioMode::Rx,
        })?;

        rfm.preamble(3)?;

        rfm.packet(PacketConfig {
            format: PacketFormat::Fixed(20),
            dc: PacketDc::None,
            crc: false,
            filtering: PacketFiltering::None,
            interpacket_rx_delay: InterPacketRxDelay::Delay2Bits,
            auto_rx_restart: true,
        })?;

        let irqevents = gpio
            .get_line(conf.interrupt_pin.ok_or(SparsnasRadioError::Conf)?)?
            .events(
                LineRequestFlags::INPUT,
                EventRequestFlags::RISING_EDGE,
                "interrupt-pin",
            )?;

        Ok(SparsnasRadio {
            rfm,
            irqevents,
            serial,
            pulses_per_kwh: conf.pulses_per_kwh,
        })
    }

    pub async fn recvloop(
        mut self,
        metrics: Arc<Mutex<SparsnasMetrics>>,
    ) -> Result<(), SparsnasRadioError> {
        self.rfm.mode(rfm69::registers::Mode::Receiver)?;

        let decoder = SparsnasDecoder::new(self.serial);
        let mut irqfd = AsyncFd::new(self.irqevents)?;

        let mut last_seq: Option<u16> = None;
        loop {
            trace!("Waiting for event...");
            irqfd.readable().await?.clear_ready();
            trace!("Got event!");
            irqfd.get_mut().get_event()?;
            let mut buffer = [0; 20];
            trace!("receving packet...");
            self.rfm.recv(&mut buffer)?;
            trace!("got packet...");

            let metrics = metrics.lock().unwrap();

            match decoder.decode(&buffer) {
                Ok(pkt) => {
                    info!(
                        "Packet: {:?} -- power: {}",
                        pkt,
                        pkt.power(self.pulses_per_kwh)
                    );
                    metrics.pulses.store(pkt.pulse_count, Ordering::SeqCst);
                    metrics
                        .current_power
                        .store(pkt.power(self.pulses_per_kwh), Ordering::SeqCst);

                    metrics.last_packet_timestamp.store(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        Ordering::SeqCst,
                    );

                    metrics.rcv_packets.fetch_add(1, Ordering::SeqCst);

                    if let Some(seq) = last_seq {
                        let missed = pkt.packet_seq.wrapping_sub(seq.wrapping_add(1));
                        if missed > 0 {
                            metrics
                                .missed_packets
                                .fetch_add(missed.into(), Ordering::SeqCst);
                        }
                    }
                    last_seq = Some(pkt.packet_seq);
                }
                Err(SparsnasDecodeError::BadCRC) => {
                    metrics.bad_crc_errors.fetch_add(1, Ordering::SeqCst);
                    debug!("Bad CRC: {:?}", buffer)
                }
                Err(e) => {
                    metrics.decode_errors.fetch_add(1, Ordering::SeqCst);
                    debug!("Decode error: {:?} of {:?}", e, buffer)
                }
            }

            self.rfm.mode(rfm69::registers::Mode::Receiver)?;
        }
    }
}
