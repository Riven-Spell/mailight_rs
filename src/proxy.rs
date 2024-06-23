use crate::jvs_parser::{JVSPacket, SegaJVSReader};
use crate::sega_led::LEDCommand;
use anyhow::Result;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

const BAUD_RATE: u32 = 115200;
const BOARD_INFO: &str = "15070-04";

fn read_and_retry(reader: &mut impl Read, buf: &mut [u8]) -> std::io::Result<()> {
    loop {
        match reader.read_exact(buf) {
            Err(err) => {
                if err.kind() != std::io::ErrorKind::TimedOut {
                    return Err(err);
                }
            }
            Ok(_) => return Ok(()),
        };
    }
}

fn mitm_packet(
    jvs_request: &JVSPacket,
    request_to_led: &mut LEDCommand,
    fix_rbg: bool,
) -> Option<JVSPacket> {
    match request_to_led {
        LEDCommand::GetBoardInfoCommand(ref mut buf) => {
            buf.clear();
            buf.extend_from_slice(BOARD_INFO.as_bytes());
            buf.push(255);
            buf.push(1);
            let mut response = JVSPacket {
                source_id: jvs_request.dest_id,
                dest_id: jvs_request.source_id,
                ..Default::default()
            };
            request_to_led.serialize_reply_to_jvs(&mut response);
            return Some(response);
        }
        LEDCommand::SetLED {
            ref mut g,
            ref mut b,
            ..
        }
        | LEDCommand::SetMultiLED {
            ref mut g,
            ref mut b,
            ..
        }
        | LEDCommand::SetMultiLEDFade {
            ref mut g,
            ref mut b,
            ..
        } => {
            if fix_rbg {
                std::mem::swap(b, g);
            }
        }
        _ => (),
    };
    None
}

pub fn proxy(
    alls_port: PathBuf,
    led_port: PathBuf,
    fix_rbg: bool,
    log_traffic: bool,
) -> Result<()> {
    let mut alls_reader_port = serialport::new(alls_port.to_str().unwrap(), BAUD_RATE)
        .timeout(Duration::from_secs(1))
        .open()?;
    let alls_writer = Mutex::new(alls_reader_port.try_clone()?);
    let mut alls_reader = BufReader::new(&mut alls_reader_port);

    let mut led_reader_port = serialport::new(led_port.to_str().unwrap(), BAUD_RATE)
        .timeout(Duration::from_secs(1))
        .open()?;
    let mut led_writer = led_reader_port.try_clone()?;
    let mut led_reader = BufReader::new(&mut led_reader_port);

    std::thread::scope(|scope| {
        // Passthrough responses from the LED board.
        scope.spawn(|| -> Result<()> {
            let mut jvs_reader = SegaJVSReader::default();
            let mut buf = [0u8; 1];
            loop {
                read_and_retry(&mut led_reader, &mut buf)?;
                if let Some(packet) = jvs_reader.read_byte(buf[0]) {
                    let mut buffer = Vec::new();
                    packet.serialize(&mut buffer);
                    alls_writer.lock().unwrap().write_all(&buffer)?;
                }
            }
        });

        // Read from the ALLS and proxy to the LED board.
        scope.spawn(|| -> Result<()> {
            let mut buf = [0u8; 1];
            let mut jvs_reader = SegaJVSReader::default();
            let mut send_buffer = Vec::new();
            loop {
                read_and_retry(&mut alls_reader, &mut buf)?;
                if let Some(packet) = jvs_reader.read_byte(buf[0]) {
                    if log_traffic {
                        tracing::info!(
                            "Got packet: src {} dst {} len {}",
                            packet.source_id,
                            packet.dest_id,
                            packet.payload.len()
                        );
                    }
                    match LEDCommand::parse(packet) {
                        Ok(mut cmd) => {
                            if log_traffic {
                                tracing::info!("LED command: {:?}", cmd);
                            }
                            if let Some(mut override_response) =
                                mitm_packet(packet, &mut cmd, fix_rbg)
                            {
                                send_buffer.clear();
                                override_response.serialize(&mut send_buffer);
                                alls_writer.lock().unwrap().write_all(&send_buffer)?;
                                continue;
                            }
                            cmd.serialize_to_jvs(packet);
                        }
                        Err(err) => {
                            tracing::error!("Couldn't parse: {:?}", err);
                        }
                    };
                    send_buffer.clear();
                    packet.serialize(&mut send_buffer);
                    led_writer.write_all(&send_buffer)?;
                }
            }
        });
    });
    Ok(())
}
