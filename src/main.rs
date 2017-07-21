#![feature(slice_patterns)]

#[macro_use]
extern crate clap;
use clap::{App, AppSettings, Arg, SubCommand, ArgMatches};

extern crate serialport;
use serialport::prelude::*;

use std::result::Result;
use std::convert::From;
use std::time::Duration;
use std::thread;
use std::io;
use std::io::Write;
use std::collections::HashMap;

mod protocol;
mod bootloader;

use bootloader::{Bootloader, Register};

#[derive(Debug)]
enum Error {
    Serial(serialport::Error),
    IO(io::Error),
    Protocol(protocol::Error),
    Connection,
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

fn connect(port_name: &str, baud_rate: usize) -> Result<Box<SerialPort>, Error>  {
    let settings = SerialPortSettings {
        baud_rate: BaudRate::from_speed(baud_rate),
        data_bits: DataBits::Eight,
        flow_control: FlowControl::None,
        parity: Parity::None,
        stop_bits: StopBits::One,
        timeout: Duration::from_secs(3),
    };

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

    fn connect_attempt(port_name: &str, settings: &SerialPortSettings, delay: bool) -> Result<(), Error> {
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
                    io::stdout().flush()?;
                    last_error = Err(error);
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }

        last_error
    }

    {
        serialport::open_with_settings(port_name, &settings)?;
    }

    print!("Connecting...");
    io::stdout().flush()?;

    let mut delay = false;
    let mut reset = false;
    for _ in 0..10 {
        if !reset {
            let mut port = serialport::open_with_settings(port_name, &settings)?;
            target_reset(&mut port, delay)?;
            reset = true;
        }

        match connect_attempt(port_name, &settings, delay) {
            Ok(_) => {
                let mut port = serialport::open_with_settings(port_name, &settings)?;
                port.set_timeout(Duration::from_secs(3))?;
                println!("");
                return Ok(port);
            },
            Err(_) => {
                if !delay {
                    delay = !delay;
                    reset = false;
                }
            }
        }
    }

    println!("");
    Err(Error::Connection)
}

fn build_cli() -> App<'static, 'static> {
    App::new("esptool")
        .author("Robby Madruga <robbymadruga@gmail.com>")
        .version("v0.0.1")
        .setting(AppSettings::SubcommandRequired)
        .arg(Arg::with_name("CHIP")
            .takes_value(true)
            .possible_values(&["auto", "esp8266", "esp32"])
            .long("chip")
        )
        .arg(Arg::with_name("PORT")
            .takes_value(true)
            .long("port")
        )
        .arg(Arg::with_name("BAUD")
            .takes_value(true)
            .long("baud")
            .required(false)
        )
        .subcommand(SubCommand::with_name("load_ram").about("Download an image to RAM and execute"))
        .subcommand(SubCommand::with_name("dump_mem").about("Dump arbitrary memory to disk"))
        .subcommand(SubCommand::with_name("read_mem").about("Read arbitrary memory location"))
        .subcommand(SubCommand::with_name("write_mem").about("Read-modify-write to arbitrary memory location"))
        .subcommand(SubCommand::with_name("write_flash").about("Write a binary blob to flash"))
        .subcommand(SubCommand::with_name("run").about("Run application code in flash"))
        .subcommand(SubCommand::with_name("image_info").about("Dump headers from an application image"))
        .subcommand(SubCommand::with_name("make_image").about("Create an application image from binary files"))
        .subcommand(SubCommand::with_name("elf2image").about("Create an application image from ELF file"))
        .subcommand(SubCommand::with_name("read_mac").about("Read MAC address from OTP ROM"))
        .subcommand(SubCommand::with_name("chip_id").about("Read Chip ID from OTP ROM"))
        .subcommand(SubCommand::with_name("flash_id").about("Read SPI flash manufacturer and device ID"))
        .subcommand(SubCommand::with_name("read_flash_status").about("Read SPI flash status register"))
        .subcommand(SubCommand::with_name("write_flash_status").about("Write SPI flash status register"))
        .subcommand(SubCommand::with_name("read_flash").about("Read SPI flash content"))
        .subcommand(SubCommand::with_name("verify_flash").about("Verify a binary blob against flash"))
        .subcommand(SubCommand::with_name("erase_flash").about("Perform Chip Erase on SPI flash"))
        .subcommand(SubCommand::with_name("erase_region").about("Erase a region of the flash"))
}

fn read_mac(port: &mut Box<SerialPort>, args: &ArgMatches) -> Result<(), Error> {
    let data = [port.read_efuse(2)?, port.read_efuse(1)?];
    let mac = [
        (data[0] >> 8) as u8,
        (data[0]) as u8,
        (data[1] >> 24) as u8,
        (data[1] >> 16) as u8,
        (data[1] >> 8) as u8,
        (data[1]) as u8,        
    ];
    println!("MAC: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
    Ok(())
}

fn main() {
    let args = build_cli().get_matches();
    
    let port_name = args.value_of("PORT").unwrap_or("/dev/ttyUSB0");
    let baud_rate = value_t!(args.value_of("BAUD"), usize)
                        .unwrap_or_else(|e| match e.kind {
                            clap::ErrorKind::ArgumentNotFound => 115200,
                            _ => e.exit(),
                        });

    let command = args.subcommand_name().unwrap();
    let command_args = args.subcommand_matches(command).unwrap();

    match connect(port_name, baud_rate) {
        Ok(mut port) => {
            let command_handler: fn(&mut Box<SerialPort>, &ArgMatches) -> Result<(), Error> = match command {
                "read_mac" => read_mac,
                _ => unimplemented!(),
            };
            command_handler(&mut port, command_args).unwrap();
        },
        Err(Error::Serial(error)) => {
            println!("Serial Error: {}", error);
        },
        Err(Error::IO(error)) => {
            println!("IO Error: {}", error);
        },
        Err(Error::Protocol(error)) => {
            println!("Protocol Error: {:?}", error);
        },
        Err(Error::Connection) => {
            println!("Connection Error: Unable to connect to device");
        }
    }
}