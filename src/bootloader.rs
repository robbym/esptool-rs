use std::io::{Read, Write};
use std::convert::Into;

use super::protocol;
use super::protocol::{Protocol, Opcode, Error};

pub enum Register {
    UartDataReg,
    EFuseRegBase,
}

impl Into<u32> for Register {
    fn into(self) -> u32 {
        match self {
            Register::UartDataReg => 0x60000078,
            Register::EFuseRegBase => 0x6001a000,
        }
    }
}

pub trait Bootloader: Read + Write {
    fn enable_flash(&mut self, hspi: u32) -> Result<(), Error> {
        let data = [
            (hspi as u8),
            (hspi >> 8) as u8,
            (hspi >> 16) as u8,
            (hspi >> 24) as u8,
            0, 0, 0, 0,
        ];
        let request = protocol::slip_encode(
            protocol::create_request(Opcode::SPIAttach, &data)
        );
        self.send_packet(&request)?;
        self.recv_packet(Opcode::SPIAttach)?;
        Ok(())
    }

    fn read_efuse(&mut self, index: u32) -> Result<u32, Error> {
        self.read_reg(Register::EFuseRegBase, 4*index)
    }

    fn read_reg(&mut self, reg: Register, offset: u32) -> Result<u32, Error> {
        let mut reg: u32 = reg.into();
        reg += offset;
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
        for _ in 0..7 {self.recv_packet(Opcode::SyncFrame)?;}
        Ok(())
    }
}

impl<T> Bootloader for T where T: Read + Write + ?Sized {}