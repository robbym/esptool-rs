use std::io;
use std::io::{Read, Write};
use std::convert::{From, Into};

pub(crate) trait Protocol: Read + Write {
    fn frame_check(&mut self) -> Result<(), Error> {
        let mut data = [0u8; 1];

        self.read_exact(&mut data)?;
        if data[0] != 0xC0 {
            return Err(Error::SLIPFrame);
        }

        Ok(())
    }

    fn recv_bytes(&mut self, packet: &mut Vec<u8>, num: usize) -> Result<(), Error> {
        let mut data = [0u8; 1];

        for _ in 0..num {
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

    fn try_recv(&mut self)-> Result<Packet, Error> {
        let mut packet = Vec::new();

        self.frame_check()?;

        println!("FRAME: Check Start");

        self.recv_bytes(&mut packet, 8)?;
        println!("FRAME: Recv Bytes");

        if packet[0] != 0x01 {
            return Err(Error::Direction);
        }

        println!("Command: {}", packet[1]);

        let size = ((packet[2] as u16) | ((packet[3] as u16) << 8)) as usize;

        println!("Size: {}", size);

        self.recv_bytes(&mut packet, size)?;
        println!("FRAME: Recv Body {:?}", &packet[8..8+size]);

        if packet[packet.len()-2] != 0x00 {
            return Err(Error::Failure(packet[packet.len()-1]));
        }

        self.frame_check()?;
        println!("FRAME: Check End");

        Ok(Packet(packet))
    }

    fn recv_packet(&mut self, opcode: Opcode) -> Result<Packet, Error> {
        let mut count = 0;
        let opcode: u8 = opcode.into();
        loop {
            match self.try_recv() {
                Ok(packet) => {
                    if packet.command() == opcode.into() {
                        return Ok(packet);
                    }
                },
                Err(e) => {
                    if count < 100 {count += 1;}
                    else {return Err(e);}
                },
            }
        }
    }

    fn send_packet(&mut self, packet: &SLIPPacket) -> Result<(), Error> {
        let &SLIPPacket(ref packet) = packet;
        self.write_all(&packet)?;
        Ok(())
    }
}

impl<T> Protocol for T where T: Read + Write + ?Sized {}

#[derive(Debug)]
pub enum Error {
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

pub(crate) enum Opcode {
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
pub(crate) struct Packet(pub(crate) Vec<u8>);
impl Packet {
    pub fn command(&self) -> u8 {
        let &Packet(ref packet) = self;
        packet[1]
    }

    pub fn size(&self) -> usize {
        let &Packet(ref packet) = self;
        (packet[2] as usize) | ((packet[3] as usize) << 8)
    }

    pub fn value(&self) -> &[u8] {
        let &Packet(ref packet) = self;
        &packet[4..8]
    }

    pub fn body(&self) -> &[u8] {
        let &Packet(ref packet) = self;
        &packet[8..self.size()+8]
    }
}


#[derive(Clone, PartialEq)]
pub(crate) struct SLIPPacket(Vec<u8>);

pub(crate) fn create_request(command: Opcode, body: &[u8]) -> Packet {
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


pub(crate) fn slip_encode(packet: Packet) -> SLIPPacket {
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