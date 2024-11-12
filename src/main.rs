mod jvs_parser;
mod proxy;
mod sega_led;

#[cfg(test)]
mod test;
mod led_pwm;

use crate::jvs_parser::JVSPacket;
use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Opts {
    File {
        path: PathBuf,
    },
    Proxy {
        alls_port: PathBuf,
        led_port: PathBuf,
        #[structopt(short, long, help = "Fix RGB vs RBG mixup")]
        fix_rbg: bool,
        #[structopt(short, long, help = "Log packets as they are sent")]
        log_traffic: bool,

        // pwm shenanigans
        #[structopt(short, long, help="Format: `<pwmchip#>-<pwm#>`, e.g. 0-3, 1-4. Accepts multiple arguments. Overlaps between FET pins average.")]
        ring: Option<Vec<String>>,
        #[structopt(short, long, help="Format: `<pwmchip#>-<pwm#>`, e.g. 0-3, 1-4. Accepts multiple arguments. Overlaps between FET pins average.")]
        side: Option<Vec<String>>,
        #[structopt(short, long, help="Format: `<pwmchip#>-<pwm#>`, e.g. 0-3, 1-4. Accepts multiple arguments. Overlaps between FET pins average.")]
        chassis: Option<Vec<String>>,
    },
}

fn parse_file(path: PathBuf) -> Result<()> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut jvs = jvs_parser::SegaJVSReader::default();
    let mut buf = Vec::new();
    let mut new_buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    for byte in &buf {
        if let Some(packet) = jvs.read_byte(*byte) {
            tracing::info!(
                "Got packet: src {} dst {} len {}",
                packet.source_id,
                packet.dest_id,
                packet.payload.len()
            );
            match sega_led::LEDCommand::parse(packet) {
                Ok(cmd) => {
                    tracing::info!("LED command: {:?}", cmd);
                    let mut new_pkt = JVSPacket::new(packet.source_id, packet.dest_id);
                    cmd.serialize_to_jvs(&mut new_pkt);
                    new_pkt.serialize(&mut new_buf);
                }
                Err(err) => tracing::error!("Couldn't parse: {:?}", err),
            }
        }
    }
    assert_eq!(buf, new_buf);
    Ok(())
}

fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let opts = Opts::from_args();

    let result = match opts {
        Opts::File { path } => parse_file(path),
        Opts::Proxy {
            alls_port,
            led_port,
            fix_rbg,
            log_traffic,
            pwm_chip,
            ring,
            side,
            chassis,
        } => crate::proxy::proxy(alls_port, led_port, fix_rbg, log_traffic,
                                 led_pwm::create_config(ring, side, chassis, pwm_chip)),
    };
    if let Err(err) = result {
        tracing::error!("Error: {:?}", err);
    }
}
