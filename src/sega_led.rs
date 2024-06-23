use crate::jvs_parser::JVSPacket;
use anyhow::{bail, Result};
use num_enum::TryFromPrimitive;

macro_rules! verbatim_parse {
    ($e:ident, $b:ident) => {
        Ok(LEDCommand::$e(without_command(&$b.payload)))
    };
}

// Buffer following the command opcode.
fn without_command(buf: &[u8]) -> Vec<u8> {
    if buf.is_empty() {
        return Vec::new();
    }
    Vec::from(&buf[1..])
}

#[derive(Debug, TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum LEDCommandType {
    Reset = 16,
    SetLED = 0x31,
    SetMultiLED = 0x32,
    SetMultiLEDFade = 51,
    SetDc = 63,
    UpdateDc = 59,

    SetFet = 57,
    Commit = 60,
    GetBoardInfoCommand = 240,
    GetProtocolVersionCommand = 243,
    GetBoardStatusCommand = 241,
    EepromWrite = 123,
    EepromRead = 124,
    SetTimeout = 17,
}

#[derive(Debug, PartialEq)]
pub enum LEDCommand {
    Reset,
    SetLED {
        index: u8,
        r: u8,
        g: u8,
        b: u8,
    },
    SetMultiLED {
        start: u8,
        end: u8,
        skip: u8,
        r: u8,
        g: u8,
        b: u8,
        speed: u8,
    },
    SetMultiLEDFade {
        start: u8,
        end: u8,
        skip: u8,
        r: u8,
        g: u8,
        b: u8,
        speed: u8,
    },
    SetDc(Vec<u8>),
    UpdateDc(Vec<u8>),
    SetFet(Vec<u8>),
    Commit,
    GetBoardInfoCommand(Vec<u8>),
    GetProtocolVersionCommand(Vec<u8>),
    GetBoardStatusCommand(Vec<u8>),
    EepromWrite(Vec<u8>),
    EepromRead(Vec<u8>),
    SetTimeout(Vec<u8>),
}
impl LEDCommand {
    pub fn parse(packet: &JVSPacket) -> Result<Self> {
        if packet.payload.is_empty() {
            bail!("Message payload was empty");
        }
        let command_type = LEDCommandType::try_from(packet.payload[0])?;
        match command_type {
            LEDCommandType::Reset => Ok(LEDCommand::Reset),
            LEDCommandType::SetLED => Ok(LEDCommand::SetLED {
                index: packet.payload[1],
                r: packet.payload[2],
                g: packet.payload[3],
                b: packet.payload[4],
            }),
            LEDCommandType::SetMultiLED => Ok(LEDCommand::SetMultiLED {
                start: packet.payload[1],
                end: packet.payload[2],
                skip: packet.payload[3],
                r: packet.payload[4],
                g: packet.payload[5],
                b: packet.payload[6],
                speed: packet.payload[7],
            }),
            LEDCommandType::SetMultiLEDFade => Ok(LEDCommand::SetMultiLEDFade {
                start: packet.payload[1],
                end: packet.payload[2],
                skip: packet.payload[3],
                r: packet.payload[4],
                g: packet.payload[5],
                b: packet.payload[6],
                speed: packet.payload[7],
            }),
            // Verbatim passthrough
            LEDCommandType::GetBoardInfoCommand => verbatim_parse!(GetBoardInfoCommand, packet),
            LEDCommandType::SetDc => verbatim_parse!(SetDc, packet),
            LEDCommandType::UpdateDc => verbatim_parse!(UpdateDc, packet),
            LEDCommandType::SetFet => verbatim_parse!(SetFet, packet),
            LEDCommandType::Commit => Ok(LEDCommand::Commit),
            LEDCommandType::GetProtocolVersionCommand => {
                verbatim_parse!(GetProtocolVersionCommand, packet)
            }
            LEDCommandType::GetBoardStatusCommand => verbatim_parse!(GetBoardStatusCommand, packet),
            LEDCommandType::EepromWrite => verbatim_parse!(EepromWrite, packet),
            LEDCommandType::EepromRead => verbatim_parse!(EepromRead, packet),
            LEDCommandType::SetTimeout => verbatim_parse!(SetTimeout, packet),
        }
    }

    pub fn get_type(&self) -> LEDCommandType {
        match self {
            LEDCommand::Reset => LEDCommandType::Reset,
            LEDCommand::SetLED { .. } => LEDCommandType::SetLED,
            LEDCommand::SetMultiLED { .. } => LEDCommandType::SetMultiLED,
            LEDCommand::SetMultiLEDFade { .. } => LEDCommandType::SetMultiLEDFade,
            LEDCommand::Commit => LEDCommandType::Commit,
            LEDCommand::SetDc(_) => LEDCommandType::SetDc,
            LEDCommand::UpdateDc(_) => LEDCommandType::UpdateDc,
            LEDCommand::SetFet(_) => LEDCommandType::SetFet,
            LEDCommand::GetBoardInfoCommand(_) => LEDCommandType::GetBoardInfoCommand,
            LEDCommand::GetProtocolVersionCommand(_) => LEDCommandType::GetProtocolVersionCommand,
            LEDCommand::GetBoardStatusCommand(_) => LEDCommandType::GetBoardStatusCommand,
            LEDCommand::EepromWrite(_) => LEDCommandType::EepromWrite,
            LEDCommand::EepromRead(_) => LEDCommandType::EepromRead,
            LEDCommand::SetTimeout(_) => LEDCommandType::SetTimeout,
        }
    }

    fn serialize_cmd_body(&self, buf: &mut Vec<u8>) {
        match self {
            LEDCommand::SetLED { index, r, g, b } => buf.extend_from_slice(&[*index, *r, *g, *b]),
            LEDCommand::SetMultiLED {
                start,
                end,
                skip,
                r,
                g,
                b,
                speed,
            } => buf.extend_from_slice(&[*start, *end, *skip, *r, *g, *b, *speed]),
            LEDCommand::SetMultiLEDFade {
                start,
                end,
                skip,
                r,
                g,
                b,
                speed,
            } => buf.extend_from_slice(&[*start, *end, *skip, *r, *g, *b, *speed]),
            LEDCommand::Reset => (),
            LEDCommand::Commit => (),
            LEDCommand::SetDc(data)
            | LEDCommand::UpdateDc(data)
            | LEDCommand::SetFet(data)
            | LEDCommand::GetBoardInfoCommand(data)
            | LEDCommand::GetProtocolVersionCommand(data)
            | LEDCommand::GetBoardStatusCommand(data)
            | LEDCommand::EepromWrite(data)
            | LEDCommand::EepromRead(data)
            | LEDCommand::SetTimeout(data) => buf.extend_from_slice(data),
        };
    }

    pub fn serialize_reply(&self, buf: &mut Vec<u8>) {
        buf.push(1); // Status
        buf.push(self.get_type() as u8);
        buf.push(1); // Report
        self.serialize_cmd_body(buf);
    }

    pub fn serialize_reply_to_jvs(&self, jvs_packet: &mut JVSPacket) {
        jvs_packet.payload.clear();
        self.serialize_reply(&mut jvs_packet.payload)
    }

    pub fn serialize(&self, buf: &mut Vec<u8>) {
        buf.push(self.get_type() as u8);
        self.serialize_cmd_body(buf);
    }

    pub fn serialize_to_jvs(&self, jvs_packet: &mut JVSPacket) {
        jvs_packet.payload.clear();
        self.serialize(&mut jvs_packet.payload)
    }
}
