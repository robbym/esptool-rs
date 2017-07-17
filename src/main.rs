extern crate clap;
use clap::{App, Arg};

extern crate serialport;

use std::result::Result;
use std::convert::From;

#[derive(Debug)]
enum Error {
    Serial(serialport::Error),
    Unknown,
}

impl From<serialport::Error> for Error {
    fn from(error: serialport::Error) -> Error {
        Error::Serial(error)
    }
}

fn flash_device(port: &str) -> Result<(), Error>  {
    let serial = serialport::open(port)?;
    Ok(())
}

fn main() {
    let args = App::new("esptool")
                .arg(Arg::with_name("port")
                    .required(true)
                )
                .get_matches();
    
    let port = args.value_of("port").unwrap();

    for port in serialport::available_ports().unwrap() {
        println!("PORT: {}", port.port_name);
    }

    match flash_device(port) {
        Err(error) => {println!("Error Occurred: {:?}", error)},
        _ => {},
    }
}
