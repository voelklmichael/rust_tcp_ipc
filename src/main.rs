use std::io::{Read, Result, Write};
use std::net::{TcpStream, ToSocketAddrs};

#[macro_use]
extern crate enum_primitive_derive;
#[macro_use]
extern crate log;

use num_traits::FromPrimitive;
use simplelog::*;

const BUFFER_SIZE: usize = 512;
use std::fmt::Debug;
trait ProtocolTrait: Debug {
    type Commands: Debug + Send + Clone + Copy + 'static;
    type States: Debug + Send + Clone + 'static;
    const HEADER_SIZE: usize;
    fn new() -> Self;
    fn get_default_state() -> Self::States;
    fn immediate_response_is_necessary(
        command: Self::Commands,
        current_state: Self::States,
    ) -> Option<String>;
    fn parse_message(
        &mut self,
        unparsed_messages: &mut Vec<u8>,
        parsed_messages: &mut Vec<(Self::Commands, Vec<u8>)>,
    ) {
        if let Some((size, command)) = self.get_header() {
            if unparsed_messages.len() < size {
                // nothing to do
            } else {
                // parse message
                let new_message = (command, unparsed_messages[0..size].to_vec());
                info!("message parsed: {:?}", new_message);
                parsed_messages.push(new_message);
                self.update_header(None);
                for _ in 0..size {
                    unparsed_messages.remove(0);
                }
                // recursively call again to parse remaining messages
                self.parse_message(unparsed_messages, parsed_messages)
            }
        } else {
            if unparsed_messages.len() < Self::HEADER_SIZE {
                // nothing to do
            } else {
                // parse header
                self.update_header(Some(&unparsed_messages[0..Self::HEADER_SIZE]));
                for _ in 0..Self::HEADER_SIZE {
                    unparsed_messages.remove(0);
                }
                // recursively call again to parse remaining messages
                self.parse_message(unparsed_messages, parsed_messages)
            }
        }
    }
    fn update_header(&mut self, header: Option<&[u8]>);
    fn get_header(&self) -> Option<(usize, Self::Commands)>;
}

#[derive(Debug, Clone, Copy, Primitive)]
enum ProtocolExampleCommands {
    Unknown = 0,
    Start = 1,
    Stop = 2,
    Pause = 3,
    Continue = 4,
    Error = 999,
    QueryIsBusy = 12,
}
#[derive(Debug, Clone)]
enum ProtocolExampleStates {
    WaitingForRequest,
    Working(String),
}

#[derive(Debug)]
struct ProtocolExample {
    header: Option<(usize, ProtocolExampleCommands)>,
}

impl ProtocolTrait for ProtocolExample {
    type Commands = ProtocolExampleCommands;
    type States = ProtocolExampleStates;
    const HEADER_SIZE: usize = 6;
    fn new() -> Self {
        ProtocolExample { header: None }
    }
    fn get_default_state() -> Self::States {
        ProtocolExampleStates::WaitingForRequest
    }
    fn immediate_response_is_necessary(
        command: Self::Commands,
        current_state: Self::States,
    ) -> Option<String> {
        use self::ProtocolExampleCommands::*;
        use self::ProtocolExampleStates::*;
        match command {
            Unknown => None,
            Start => None,
            Stop => None,
            Pause => None,
            Continue => None,
            Error => None,
            QueryIsBusy => match current_state {
                WaitingForRequest => None,
                Working(message) => Some(message),
            },
        }
    }
    fn update_header(&mut self, header: Option<&[u8]>) {
        if let Some(h) = header {
            const SIZE_BYTES: usize = 3;
            assert!(SIZE_BYTES < Self::HEADER_SIZE);
            let mut payload_size = 0usize;
            for i in 0..SIZE_BYTES {
                payload_size += h[i] as usize * 256u32.pow((SIZE_BYTES - 1 - i) as u32) as usize;
            }
            let mut command = 0u32;
            for i in 0..(Self::HEADER_SIZE - SIZE_BYTES) {
                command += h[i + SIZE_BYTES] as u32
                    * 256u32.pow((Self::HEADER_SIZE - SIZE_BYTES - 1 - i) as u32);
            }
            let command = if let Some(x) = ProtocolExampleCommands::from_u32(command) {
                x
            } else {
                debug!(
                    "received unknown header {:?}, parsed command number{:?}",
                    header, command
                );
                ProtocolExampleCommands::Error
            };

            self.header = Some((payload_size, command));
        } else {
            self.header = None;
        }
    }
    fn get_header(&self) -> Option<(usize, Self::Commands)> {
        self.header
    }
}

#[derive(Debug)]
struct Client<Protocol: ProtocolTrait> {
    incoming_message_queue: std::sync::Arc<
        std::sync::Mutex<(
            Vec<(<Protocol as ProtocolTrait>::Commands, Vec<u8>)>,
            <Protocol as ProtocolTrait>::States,
        )>,
    >,
}

impl<Protocol: ProtocolTrait> Client<Protocol> {
    fn new() -> Self {
        Client {
            incoming_message_queue: std::sync::Arc::new(std::sync::Mutex::new((
                Vec::new(),
                <Protocol as ProtocolTrait>::get_default_state(),
            ))),
        }
    }
    fn connect<T: ToSocketAddrs>(
        &mut self,
        socket_addresses: T,
        timeout_time: std::time::Duration,
    ) -> Result<()> {
        let mut socket_addresses = socket_addresses.to_socket_addrs()?;
        let mut error =
            std::io::Error::new(std::io::ErrorKind::Other, "Socket Address list is empty");
        // connect
        let mut stream = loop {
            if let Some(socket_address) = socket_addresses.next() {
                info!("try to connect to {:?}", socket_address);
                match TcpStream::connect_timeout(&socket_address, timeout_time) {
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
        // start read thread
        let messages_inside = self.incoming_message_queue.clone();
        std::thread::spawn(move || {
            let mut buffer = [0; BUFFER_SIZE as usize];
            let mut unparsed_messages = Vec::with_capacity(BUFFER_SIZE);
            let mut protocol = Protocol::new();
            loop {
                match stream.read(&mut buffer) {
                    Ok(n) => {
                        if n == 0 {
                            info!("package of size 0 received, so shuting down");
                            break;
                        } else {
                            unparsed_messages.extend_from_slice(&buffer[0..n]);
                            let mut parsed_messages = Vec::new();
                            Protocol::parse_message(
                                &mut protocol,
                                &mut unparsed_messages,
                                &mut parsed_messages,
                            );
                            if !parsed_messages.is_empty() {
                                if let Ok(mut inner_data) = messages_inside.lock() {
                                    for new_message in parsed_messages {
                                        if let Some(message) = <Protocol as ProtocolTrait>::immediate_response_is_necessary(
                                            new_message.0, inner_data.1.clone())
                                        {
                                            info!("immediate response necessary, answering: {:?}", message);
                                            stream.write(message.as_bytes()).unwrap();

                                        } else {
                                        inner_data.0.push(new_message);
                                    }
                                    }
                                } else {
                                    error!("Failed to lock mutex in read thread");
                                    break;
                                }
                            }
                        }
                    }
                    Err(error) => error!("error during read: {:?}", error),
                }
            }
        });

        Ok(())
    }
}
fn main() {
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();
    let mut client = Client::<ProtocolExample>::new();
    client
        .connect("127.0.0.1:8080", std::time::Duration::from_millis(200))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(202));
    {
        let mut messages = client.incoming_message_queue.lock().unwrap();
        while let Some(message) = messages.0.pop() {
            println!("{:?}", message);
        }
    }
    println!("--------");
    std::thread::sleep(std::time::Duration::from_millis(200));
    {
        let messages = client.incoming_message_queue.lock().unwrap();
        for message in messages.0.iter() {
            println!("{:?}", message);
        }
    }
}
