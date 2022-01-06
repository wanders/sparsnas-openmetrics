mod radio;

mod sparsnasmetrics;
use sparsnasmetrics::SparsnasMetrics;

mod openmetric;

use std::fmt;
use std::net::IpAddr;
use std::process::exit;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use futures::join;
use log::trace;

use clap::Parser;
use warp::Filter;

/// RFM69 Sparsnas receiver and openmetrics exporter
#[derive(Parser)]
#[clap(version = "0.1")]
struct Opts {
    serial: u32,
    #[clap(short, long, default_value = "1000")]
    pulses_per_kwh: u32,

    #[clap(short, long, default_value = "3030")]
    openmetrics_port: u16,

    #[clap(long, default_value = "::")]
    openmetrics_listen_addr: String,

    #[clap(short, long, parse(from_occurrences))]
    verbose: u8,
}

enum MainError {
    LogInit,
    ListenAddressParse(std::net::AddrParseError),
    Sparsnas(radio::SparsnasRadioError),
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MainError::LogInit => write!(f, "Error initializing log"),
            MainError::ListenAddressParse(e) => write!(f, "Could not parse listen address: {}", e),
            MainError::Sparsnas(radio::SparsnasRadioError::NoContactWithChip) => {
                write!(f, "No contact with hardware")
            }
            MainError::Sparsnas(e) => write!(f, "Error configuring hardware: {:?}", e),
        }
    }
}

#[tokio::main]
async fn amain() -> Result<(), MainError> {
    let opts: Opts = Opts::parse();

    let bind_ip =
        IpAddr::from_str(&opts.openmetrics_listen_addr).map_err(MainError::ListenAddressParse)?;

    stderrlog::new()
        .module(module_path!())
        .verbosity(2 + opts.verbose as usize)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .map_err(|_| MainError::LogInit)?;

    let sparsnas = radio::SparsnasRadioConfig::new()
        .gpiochip_device("/dev/gpiochip0")
        .interrupt_pin(24) /* 24 == pin 18 */
        .reset_pin(5) /*  5 == pin 29 */
        .pulses_per_kwh(opts.pulses_per_kwh)
        .serial(opts.serial)
        .build()
        .map_err(MainError::Sparsnas)?;

    let metrics = Arc::new(Mutex::new(SparsnasMetrics::new()));

    let pulses = warp::path!("metrics").map({
        let metrics = metrics.clone();
        move || {
            let mut res = String::new();
            trace!("Got metrics request");
            metrics.lock().unwrap().render_metrics(&mut res);
            res
        }
    });

    let server = warp::serve(pulses).run((bind_ip, opts.openmetrics_port));

    join!(server, sparsnas.recvloop(metrics)).1.unwrap();
    Ok(())
}

fn main() {
    amain().unwrap_or_else(|err| {
        eprintln!("Error: {}", err);
        exit(1)
    })
}
