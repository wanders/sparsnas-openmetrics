use crate::openmetric::{OpenMetric, OpenMetricKind};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

#[derive(Debug)]
pub struct SparsnasMetrics {
    pub pulses: AtomicU32,
    pub current_power: AtomicU32,
    pub last_packet_timestamp: AtomicU64,
    pub rcv_packets: AtomicU32,
    pub decode_errors: AtomicU32,
    pub bad_crc_errors: AtomicU32,
    pub missed_packets: AtomicU32,
}

use std::fmt::Write;

impl SparsnasMetrics {
    pub fn new() -> SparsnasMetrics {
        SparsnasMetrics {
            pulses: AtomicU32::new(0),
            current_power: AtomicU32::new(0),
            last_packet_timestamp: AtomicU64::new(0),
            rcv_packets: AtomicU32::new(0),
            decode_errors: AtomicU32::new(0),
            bad_crc_errors: AtomicU32::new(0),
            missed_packets: AtomicU32::new(0),
        }
    }

    pub fn render_metrics(&self, res: &mut impl Write) {
        let ts = self.last_packet_timestamp.load(Ordering::SeqCst);
        OpenMetric::new(OpenMetricKind::Counter, "sparsnas_pulses")
            .help("Total number of pulses (blinks) transmitter has seen since poweron.")
            .timestamp(ts as f64)
            .value(self.pulses.load(Ordering::SeqCst) as f64)
            .render(res);

        OpenMetric::new(OpenMetricKind::Gauge, "sparsnas_power")
            .help("Instantaneous power usage.")
            .unit("Watt")
            .timestamp(ts as f64)
            .value(self.current_power.load(Ordering::SeqCst).into())
            .render(res);

        OpenMetric::new(OpenMetricKind::Counter, "sparsnas_packets")
            .help("Received packets")
            .value(self.rcv_packets.load(Ordering::SeqCst).into())
            .render(res);

        OpenMetric::new(OpenMetricKind::Counter, "sparsnas_packet_decode_errors")
            .help("Received packets that couldn't be decoded")
            .value(self.decode_errors.load(Ordering::SeqCst).into())
            .render(res);

        OpenMetric::new(OpenMetricKind::Counter, "sparsnas_bad_crc_errors")
            .help("CRC errors")
            .value(self.bad_crc_errors.load(Ordering::SeqCst).into())
            .render(res);

        OpenMetric::new(OpenMetricKind::Counter, "sparsnas_missed_packets")
            .help("Missed packets (based on packet sequence numbers)")
            .value(self.missed_packets.load(Ordering::SeqCst).into())
            .render(res);

        writeln!(res, "# EOF").unwrap();
    }
}
