use std::io::Result;
use std::net::{TcpStream, ToSocketAddrs};

#[macro_use]
extern crate log;

extern crate simplelog;
use simplelog::*;

#[derive(Debug)]
struct SomeType {}
fn connect<T: ToSocketAddrs>(socket_addresses: T) -> Result<SomeType> {
    let mut socket_addresses = socket_addresses.to_socket_addrs()?;
    let mut error = std::io::Error::new(std::io::ErrorKind::Other, "Socket Address list is empty");
    let stream = loop {
        if let Some(socket_address) = socket_addresses.next() {
            info!("try to connect to {:?}", socket_address);
            match TcpStream::connect_timeout(&socket_address, std::time::Duration::from_millis(100))
            {
                Ok(stream) => {
                    info!("connected");
                    break stream;
                }
                Err(err) => {
                    info!("Received error: {:?}", err);
                    error = err;
                }
            }
        } else {
            return Err(error);
        }
    };
    Ok(SomeType {})
}

fn main() {
    let mut config = Config::default();
    config.level = Some(Level::Info);
    config.location = Some(Level::Info);
    config.time = Some(Level::Info);
    config.target = Some(Level::Info);
    CombinedLogger::init(vec![TermLogger::new(LevelFilter::Info, config).unwrap()]).unwrap();
    error!("Bright red error");
    println!("{:?}", connect("12.0.0.1:8080"));
}
