#![feature(slice_patterns)]

extern crate clap;
use clap::{App, Arg};

extern crate serialport;
use serialport::prelude::*;

use std::result::Result;
use std::convert::From;
use std::time::Duration;
use std::thread;

mod protocol;
mod bootloader;

use bootloader::{Bootloader, Register};

#[derive(Debug)]
enum Error {
    Serial(serialport::Error),
    Protocol(protocol::Error),
    Unknown,
}

impl From<serialport::Error> for Error {
    fn from(error: serialport::Error) -> Error {
        Error::Serial(error)
    }
}

fn connect(port_name: &str) -> Result<Box<SerialPort>, Error>  {
    let settings = SerialPortSettings {
        baud_rate: BaudRate::Baud115200,
        data_bits: DataBits::Eight,
        flow_control: FlowControl::None,
        parity: Parity::None,
        stop_bits: StopBits::One,
        timeout: Duration::from_secs(3),
    };
    let mut port = serialport::open_with_settings(port_name, &settings)?;


    port.write_data_terminal_ready(false)?;
    port.write_request_to_send(true)?;
    thread::sleep(Duration::from_millis(100));
    port.write_data_terminal_ready(true)?;
    port.write_request_to_send(false)?;
    thread::sleep(Duration::from_millis(50));
    port.write_data_terminal_ready(false)?;

    port.read_to_end(&mut Vec::new());
    port.set_timeout(Duration::from_millis(100))?;

    let mut count = 0;
    loop {
        let result = port.sync();

        if count < 7 {
            count += 1;
        } else {
            if let Err(e) = result {
                return Err(Error::Protocol(e));
            }
            break;
        }
        print!(".");
        port.flush();
    }

    port.set_timeout(Duration::from_secs(3))?;

    Ok(port)
}

fn main() {
    let args = App::new("esptool")
                .arg(Arg::with_name("port")
                    .required(true)
                )
                .get_matches();
    
    let port_name = args.value_of("port").unwrap();
    let mut port = connect(port_name).unwrap();
    let value = port.read_reg(Register::UartDataReg).unwrap();
    println!("Value: {}", value);
}