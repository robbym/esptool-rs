#![feature(slice_patterns)]

extern crate clap;
use clap::{App, Arg};

extern crate serialport;
use serialport::prelude::*;

use std::result::Result;
use std::convert::From;
use std::time::Duration;
use std::thread;
use std::io;
use std::io::Write;

mod protocol;
mod bootloader;

use bootloader::{Bootloader, Register};

#[derive(Debug)]
enum Error {
    Serial(serialport::Error),
    IO(io::Error),
    Protocol(protocol::Error),
    Unknown,
}

impl From<serialport::Error> for Error {
    fn from(error: serialport::Error) -> Error {
        Error::Serial(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::IO(error)
    }
}

impl From<protocol::Error> for Error {
    fn from(error: protocol::Error) -> Error {
        Error::Protocol(error)
    }
}

fn try_sync(port: &mut Box<SerialPort>) -> Result<(), Error> {
    port.read_to_end(&mut Vec::new());
    port.flush()?;
    port.sync()?;
    port.set_timeout(Duration::from_secs(3))?;
    Ok(())
}

fn target_reset(mut port: &mut Box<SerialPort>, delay: bool) -> Result<(), Error> {
    port.write_data_terminal_ready(false)?;
    port.write_request_to_send(true)?;

    thread::sleep(Duration::from_millis(100));
    if delay {thread::sleep(Duration::from_millis(1200));}

    port.write_data_terminal_ready(true)?;
    port.write_request_to_send(false)?;

    if delay {thread::sleep(Duration::from_millis(400));}
    thread::sleep(Duration::from_millis(50));

    port.write_data_terminal_ready(false)?;

    Ok(())
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

    let connect_attempt = |delay| {
        let mut last_error = Ok(());
        for _ in 0..5 {
            let mut port = serialport::open_with_settings(port_name, &settings)?;
            port.set_timeout(Duration::from_millis(100))?;
            match try_sync(&mut port) {
                Ok(_) => {
                    break;
                },
                Err(error) => {
                    print!("{}", if delay {"_"} else {"."});
                    io::stdout().flush();
                    last_error = Err(error);
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }

        last_error
    };

    print!("Connecting...");
    std::io::stdout().flush();

    let mut delay = false;
    let mut reset = false;
    for _ in 0..10 {
        if !reset {
            let mut port = serialport::open_with_settings(port_name, &settings)?;
            target_reset(&mut port, delay)?;
            reset = true;
        }

        match connect_attempt(delay) {
            Ok(_) => {
                let mut port = serialport::open_with_settings(port_name, &settings)?;
                port.set_timeout(Duration::from_secs(3))?;
                println!("");
                return Ok(port);
            },
            Err(_) => {
                if !delay {
                    delay = true;
                    reset = false;
                }
            }
        }
    }

    Err(Error::Unknown)
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
    let value = port.read_reg(Register::UartDataReg).unwrap();
    println!("Value: {:X}", value);
}