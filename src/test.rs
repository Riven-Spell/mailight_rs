use crate::jvs_parser::{JVSPacket, SegaJVSReader};
use crate::sega_led;

#[test]
fn test_serialization() {
    let test_data = include_bytes!("./test_mode_data.bin");
    let mut jvs = SegaJVSReader::default();
    let mut new_buf = Vec::new();
    for byte in test_data {
        if let Some(packet) = jvs.read_byte(*byte) {
            let cmd = sega_led::LEDCommand::parse(packet).expect("led command");
            // Create our own version of the same packet.
            let mut new_pkt = JVSPacket::new(packet.source_id, packet.dest_id);
            cmd.serialize_to_jvs(&mut new_pkt);
            new_pkt.serialize(&mut new_buf);
        }
    }
    // Our version should look the same as the input.
    assert_eq!(test_data, new_buf.as_slice());
}
