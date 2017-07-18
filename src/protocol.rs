use std::io::{Read, Write};
use std::convert::Into;

trait Bootloader: Read + Write {
    fn send_packet(packet: &SLIPPacket) -> Result<SLIPPacket, Error> {
        
    }
}

#[derive(Debug)]
enum Error {
    SLIPFrame,
    Direction,
    Command,
    Length,
    Checksum,
    Failure(u8),
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
            FlashStart => 0x02,
            FlashData => 0x03,
            FlashFinish => 0x04,
            RAMStart => 0x05,
            RAMFinish => 0x06,
            RAMData => 0x07,
            SyncFrame => 0x08,
            WriteReg => 0x09,
            ReadReg => 0x0A,
            ConfigSPI => 0x0B,
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

fn slip_decode(encoded: SLIPPacket) -> Result<Packet, Error> {
    let SLIPPacket(mut packet) = encoded;

    if packet[0] != 0xC0 || packet[packet.len()-1] != 0xC0 {
        return Err(Error::SLIPFrame);
    }

    packet.remove(0);
    packet.pop();

    for i in (0..packet.len()-1).rev() {
        match packet[i..i+1] {
            [0xDB, 0xDC] => {packet[i] = 0xC0; packet.remove(i+1);},
            [0xDB, 0xDD] => {packet[i] = 0xDB; packet.remove(i+1);},
            _ => {},
        }
    }

    Ok(Packet(packet))
}

#[test]
fn encode_decode() {
    let data = vec![1, 2, 0xC0, 4, 5, 0xDB, 7, 8, 9, 10];
    let packet1 = create_request(Opcode::FlashStart, &data);
    let packet2 = slip_decode(slip_encode(packet1.clone())).unwrap();
    assert!(packet1 == packet2);
}