const SYNC_BYTE: u8 = 0xE0;
const ESCAPE_BYTE: u8 = 0xD0;

fn escape_and_push(input: &[u8], output: &mut Vec<u8>, checksum: &mut u8) {
    for byte in input {
        *checksum = checksum.wrapping_add(*byte);
        if *byte == SYNC_BYTE {
            output.extend_from_slice(&[ESCAPE_BYTE, byte - 1]);
        } else {
            output.push(*byte);
        }
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct JVSPacket {
    pub source_id: u8,
    pub dest_id: u8,
    pub expected_len: u8,
    pub payload: Vec<u8>,
    pub checksum: u8,
}
impl JVSPacket {
    pub fn new(source: u8, dest: u8) -> Self {
        Self {
            source_id: source,
            dest_id: dest,
            expected_len: 0,
            payload: Vec::new(),
            checksum: 0,
        }
    }

    pub fn serialize(&mut self, buf: &mut Vec<u8>) {
        buf.push(SYNC_BYTE);
        let mut checksum = 0u8;
        self.expected_len = self.payload.len() as u8;
        escape_and_push(
            &[self.dest_id, self.source_id, self.expected_len],
            buf,
            &mut checksum,
        );
        escape_and_push(&self.payload, buf, &mut checksum);
        buf.push(checksum);
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
enum ReaderState {
    #[default]
    Src,
    Dest,
    Len,
    Payload,
    PayloadEscaped,
    Checksum,
    Ready,
}

#[derive(Default)]
pub struct SegaJVSReader {
    state: ReaderState,
    packet: JVSPacket,
}
impl SegaJVSReader {
    fn reset(&mut self) -> ReaderState {
        self.packet = JVSPacket::default();
        self.state = ReaderState::Dest;
        self.state
    }

    pub fn read_byte(&mut self, mut input: u8) -> Option<&mut JVSPacket> {
        if input == SYNC_BYTE {
            self.reset();
            return None;
        }

        self.state = match self.state {
            ReaderState::Src => {
                self.packet.source_id = input;
                self.packet.checksum = self.packet.checksum.wrapping_add(input);
                ReaderState::Len
            }
            ReaderState::Dest => {
                self.packet.dest_id = input;
                self.packet.checksum = self.packet.checksum.wrapping_add(input);
                ReaderState::Src
            }
            ReaderState::Len => {
                self.packet.expected_len = input;
                self.packet.checksum = self.packet.checksum.wrapping_add(input);
                ReaderState::Payload
            }
            ReaderState::PayloadEscaped | ReaderState::Payload => 'b: {
                if input == ESCAPE_BYTE {
                    break 'b ReaderState::PayloadEscaped;
                }

                if self.state == ReaderState::PayloadEscaped {
                    input += 1;
                }

                self.packet.payload.push(input);
                self.packet.checksum = self.packet.checksum.wrapping_add(input);
                if self.packet.payload.len() == self.packet.expected_len as usize {
                    ReaderState::Checksum
                } else {
                    ReaderState::Payload
                }
            }
            ReaderState::Checksum => {
                if self.packet.checksum == input {
                    ReaderState::Ready
                } else {
                    tracing::error!("Bad checksum");
                    self.reset()
                }
            }
            ReaderState::Ready => {
                tracing::warn!("Got data in ready state: {:#04x}", input);
                ReaderState::Ready
            }
        };

        match self.state {
            ReaderState::Ready => Some(&mut self.packet),
            _ => None,
        }
    }
}
