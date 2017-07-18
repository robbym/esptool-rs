use std::io;
use std::io::{Read, Write};
use std::convert::{From, Into};

trait Protocol: Read + Write {
    fn frame_check(&mut self) -> Result<(), Error> {
        let mut data = [0u8; 1];

        self.read_exact(&mut data)?;
        if data[0] != 0xC0 {
            return Err(Error::SLIPFrame);
        }

        Ok(())
    }

    fn recv_bytes(&mut self, packet: &mut Vec<u8>, expected: usize) -> Result<(), Error> {
        let mut data = [0u8; 1];

        while packet.len() < expected {
            self.read_exact(&mut data)?;
            match data[0] {
                0xDB => {
                    self.read_exact(&mut data)?;
                    match data[0] {
                        0xDC => {packet.push(0xC0);},
                        0xDD => {packet.push(0xDB);},
                        _ => {return Err(Error::SLIPFrame);},
                    }
                },
                byte => {packet.push(byte);}
            }
        }

        Ok(())
    }

    fn recv_packet(&mut self) -> Result<Packet, Error> {
        let mut packet = Vec::new();

        self.frame_check()?;

        self.recv_bytes(&mut packet, 8)?;

        if packet[0] != 0x01 {
            return Err(Error::Direction);
        }

        let size = (packet[2] as u16) | ((packet[3] as u16) << 8);
        self.recv_bytes(&mut packet, (size + 6) as usize)?;

        self.frame_check()?;

        Ok(Packet(packet))
    }

    fn send_packet(&mut self, packet: &SLIPPacket) -> Result<(), Error> {
        let &SLIPPacket(ref packet) = packet;
        self.write_all(&packet)?;
        Ok(())
    }
}

impl<T> Protocol for T where T: Read + Write {}

#[derive(Debug)]
enum Error {
    Device(io::Error),
    SLIPFrame,
    Direction,
    Command,
    Length,
    Checksum,
    Failure(u8),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Device(error)
    }
}

enum Opcode {
    FlashStart,
    FlashData,
    FlashFinish,
    RAMStart,
    RAMFinish,
    RAMData,
    SyncFrame,
    WriteReg,
    ReadReg,
    ConfigSPI,
}

impl Into<u8> for Opcode {
    fn into(self) -> u8 {
        match self {
            Opcode::FlashStart => 0x02,
            Opcode::FlashData => 0x03,
            Opcode::FlashFinish => 0x04,
            Opcode::RAMStart => 0x05,
            Opcode::RAMFinish => 0x06,
            Opcode::RAMData => 0x07,
            Opcode::SyncFrame => 0x08,
            Opcode::WriteReg => 0x09,
            Opcode::ReadReg => 0x0A,
            Opcode::ConfigSPI => 0x0B,
        }
    }
}

#[derive(Clone, PartialEq)]
struct Packet(Vec<u8>);

#[derive(Clone, PartialEq)]
struct SLIPPacket(Vec<u8>);

fn create_request(command: Opcode, body: &[u8]) -> Packet {
    let mut packet = Vec::new();

    let len = body.len() as u16;
    let checksum: u32 = body.iter().fold(0xEF, |acc, n| acc ^ *n as u32);

    packet.push(0x00);
    packet.push(command.into());
    packet.push(len as u8);
    packet.push((len >> 8) as u8);
    packet.push(checksum as u8);
    packet.push((checksum >> 8) as u8);
    packet.push((checksum >> 16) as u8);
    packet.push((checksum >> 24) as u8);

    Packet(packet)
}


fn slip_encode(packet: Packet) -> SLIPPacket {
    let Packet(packet) = packet;

    let mut encoded = packet.iter().fold(vec![0xC0], |mut acc, n| {
        match *n {
            0xC0 => {acc.push(0xDB); acc.push(0xDC);},
            0xDB => {acc.push(0xDB); acc.push(0xDD);},
            _ => {acc.push(*n);}
        }
        acc
    });
    encoded.push(0xC0);

    SLIPPacket(encoded)
}