use std::io::{Read, Write};
use std::convert::Into;

use super::protocol;
use super::protocol::{Protocol, Opcode, Error};

pub enum Register {
    UartDataReg
}

impl Into<u32> for Register {
    fn into(self) -> u32 {
        match self {
            Register::UartDataReg => 0x60000078,
        }
    }
}

pub trait Bootloader: Read + Write {
    fn read_reg(&mut self, reg: Register) -> Result<u32, Error> {
        let reg: u32 = reg.into();
        let data = [
            (reg as u8),
            (reg >> 8) as u8,
            (reg >> 16) as u8,
            (reg >> 24) as u8,
        ];
        let request = protocol::slip_encode(
            protocol::create_request(Opcode::ReadReg, &data)
        );
        self.send_packet(&request)?;

        let response = self.recv_packet(Opcode::ReadReg)?;
        if response.size() != 4 {
            return Err(Error::Length);
        }

        let value = response.value();

        Ok(
            (value[0] as u32) |
            (value[1] as u32) << 8 |
            (value[2] as u32) << 16 |
            (value[3] as u32) << 24           
        )
    }

    fn sync(&mut self) -> Result<(), Error> {
        let mut data = vec![0x07, 0x07, 0x12, 0x20];
        data.extend_from_slice(&[0x55; 32]);
        let request = protocol::slip_encode(
            protocol::create_request(Opcode::SyncFrame, &data)
        );
        self.send_packet(&request)?;
        self.recv_packet(Opcode::SyncFrame)?;
        Ok(())
    }
}

impl<T> Bootloader for T where T: Read + Write + ?Sized {}