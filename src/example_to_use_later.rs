use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

//use num_derive::*;

//mod protocol;

//use num_traits::FromPrimitive;
//use simplelog::*;

const HEADER_SIZE: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Commands {
    Start,
    Funny,
}
fn array_to_enum(input: &[u8; 2]) -> Option<Commands> {
    use self::Commands::*;
    match input {
        [b'0', b'0'] => Some(Start),
        [b'4', b'2'] => Some(Funny),
        _ => None,
    }
}

fn main() {
    // start server thread
    std::thread::spawn(move || {
        let (mut server, socket_address) = TcpListener::bind("127.0.0.1:6666")
            .unwrap()
            .accept()
            .unwrap();
        println!("server connected to {:?}", socket_address);
        let message = [0, 0, 1, b'4', b'2', b'a'];
        for &x in &message {
            server.write(&[x]).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let message = [0, 0, 6, b'0', b'0', b'a', 0, 1, 2, 3, 4];
        for &x in &message {
            server.write(&[x]).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        println!("server thread finished");
    });
    std::thread::sleep(std::time::Duration::from_micros(10));

    // start client thread
    let mut client = TcpStream::connect("127.0.0.1:6666").expect("client unwrap");
    client
        .set_read_timeout(Some(std::time::Duration::from_micros(1)))
        .unwrap();
    let mut incoming_buffer_vec = Vec::<u8>::new();
    let mut current_command: Option<Commands> = None;
    let mut current_remaing: usize = 0;
    let mut current_message: Vec<u8> = Vec::new();

    let mut incoming_buffer = [0; 128];
    loop {
        match client.read(&mut incoming_buffer) {
            Ok(message_length) => {
                if message_length == 0 {
                    panic!("message length is zero");
                }
                let incoming_buffer = &incoming_buffer[0..message_length];
                //debug!("recv: {:?}", &incoming_buffer[0..message_length]);
                let unused_buffer = if let Some(command) = current_command {
                    if message_length < current_remaing {
                        current_message.extend_from_slice(&incoming_buffer[0..message_length]);
                        current_remaing -= message_length;
                        &[]
                    } else {
                        current_message.extend_from_slice(&incoming_buffer[0..current_remaing]);
                        received_message(command, current_message.clone());
                        current_message.clear();
                        let remaining_length_before = current_remaing;
                        current_remaing = 0; //not strictly necessary
                        current_command = None;
                        &incoming_buffer[remaining_length_before..]
                    }
                } else {
                    &incoming_buffer
                };
                incoming_buffer_vec.extend_from_slice(unused_buffer);
                if let Some((header, message)) =
                    message_slice_to_header_array(incoming_buffer_vec.as_slice())
                {
                    let (command, length) = match parse(header) {
                        Ok((command, length)) => (command, length),
                        Err((err, message)) => {
                            panic!("parse error: {:?}, incoming header: {:?}", err, message)
                        }
                    };
                    current_command = Some(command);
                    current_remaing = length;
                    current_message = message.to_vec(); // capicity can also be set already
                    current_message.reserve(length - current_message.len());
                    incoming_buffer_vec.clear();
                } else {
                    // wait for further messages
                }
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                } else {
                    panic!("read error{:?}", err)
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_micros(10));
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
}

fn received_message(command: Commands, message: Vec<u8>) {
    println!("{:?}", (command, message));
    if command == Commands::Start {
        panic!("working");
    }
}

#[derive(Debug)]
enum ParseErrors {
    CommandParseError,
}
fn parse(header: &[u8; HEADER_SIZE]) -> Result<(Commands, usize), (ParseErrors, Vec<u8>)> {
    if let Some(command) = array_to_enum(&[header[3], header[4]]) {
        let length: usize = header[0..3]
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &x)| ((x as u32) * 10u32.pow(i as u32)) as usize)
            .sum();
        Ok((command, length))
    } else {
        Err((ParseErrors::CommandParseError, header.to_vec()))
    }
}

#[inline]
fn message_slice_to_header_array(slice: &[u8]) -> Option<(&[u8; HEADER_SIZE], &[u8])> {
    if slice.len() >= HEADER_SIZE {
        Some((
            unsafe { &*(slice[0..HEADER_SIZE].as_ptr() as *const [_; HEADER_SIZE]) },
            &slice[HEADER_SIZE..],
        ))
    } else {
        None
    }
}

/*const BUFFER_SIZE: usize = 512;
use std::fmt::Debug;
/// This trait represents a message protocol.
/// The associated type "Commands" represents the possible actions like Wait, Start, Stop,  etc.
/// The associated type "State" is used to respond immediately to some commands. For example, during a long computation the client still can answer immediately, e.g. if the server queries the clients current state.
pub trait ProtocolTrait {
    type Commands: Debug + Send + Clone + Copy + 'static;
    type States: Debug + Send + Clone + 'static;
    const HEADER_SIZE: usize;
    fn new() -> Self;
    /// Define an default state which is used to initialize the client
    fn get_default_state() -> Self::States;
    /// Checks if a received command requires an immediate action and - if so - return the message which will be send to the server.
    fn immediate_response_is_necessary(
        command: Self::Commands,
        current_state: Self::States,
    ) -> Option<Vec<u8>>;
    /// Parses an received bit stream into (possibly several) messages and appends those to a given set of messages
    /// This has a default implementation
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
        } else if unparsed_messages.len() < Self::HEADER_SIZE {
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
    Working(Vec<u8>),
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
    ) -> Option<Vec<u8>> {
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
                payload_size +=
                    usize::from(h[i]) * 256u32.pow((SIZE_BYTES - 1 - i) as u32) as usize;
            }
            let mut command = 0u32;
            for i in 0..(Self::HEADER_SIZE - SIZE_BYTES) {
                command += u32::from(h[i + SIZE_BYTES])
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
                                            stream.write(message.as_slice()).unwrap();

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
*/
