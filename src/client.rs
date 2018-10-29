pub use super::protocol_buffer::*;
use super::BUFFER_SIZE;

pub use std::io::{Read, Write};
pub use std::net::{TcpListener, TcpStream, ToSocketAddrs};
pub use std::sync::mpsc::TryRecvError;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub connect_wait_time_ms: u64,
    pub read_iteration_wait_time_ns: u64,
}

#[derive(Debug)]
pub enum ReadThreadErrors<P: Protocol> {
    WriteError(std::io::Error),
    ReadError(std::io::Error),
    ImmediateMessageParseError((P::Commands, Vec<u8>)),
}
#[derive(Debug)]
pub enum ConnectErrors {
    SocketListIsEmpty,
    SocketListParseError(std::io::Error),
    TryCloneError(std::io::Error),
    ConnectionError(std::io::Error),
}
pub struct Client<P: Protocol> {
    busy_state_sender: std::sync::mpsc::Sender<P::BusyStates>,
    message_receiver: std::sync::mpsc::Receiver<Result<Message<P>, ReadThreadErrors<P>>>,
    stream: TcpStream,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BusyStateUpdateResult {
    Success,
    Disconnected,
}
#[derive(Debug)]
pub enum WriteMessageErrors {
    MessageConstructionFailed,
    MessageSendFailed(std::io::Error),
}
impl<P: Protocol> Client<P> {
    pub fn connect<T: ToSocketAddrs>(
        socket_addresses: T,
        config: Config,
    ) -> Result<Client<P>, ConnectErrors> {
        let mut socket_addresses = socket_addresses
            .to_socket_addrs()
            .map_err(ConnectErrors::SocketListParseError)?;
        let mut error = self::ConnectErrors::SocketListIsEmpty;
        // connect
        let client = loop {
            if let Some(socket_address) = socket_addresses.next() {
                debug!("try to connect to {:?}", socket_address);
                match TcpStream::connect_timeout(
                    &socket_address,
                    std::time::Duration::from_millis(config.connect_wait_time_ms),
                ) {
                    Ok(stream) => {
                        info!("connected to {:?}", socket_address);
                        break stream;
                    }
                    Err(err) => {
                        info!("Received error: {:?}", err);
                        error = ConnectErrors::ConnectionError(err);
                    }
                }
            } else {
                return Err(error);
            }
        };
        // set non-blocking
        if client.set_nonblocking(true).is_err() {
            std::io::Error::new(std::io::ErrorKind::Other, "set_nonblocking call failed");
            return Err(error);
        }
        // start read thread
        let mut client_read = client.try_clone().map_err(ConnectErrors::TryCloneError)?;
        let (message_sender, message_receiver) = std::sync::mpsc::channel();
        let (busy_state_sender, busy_state_receiver) = std::sync::mpsc::channel();
        let mut protocol = ProtocolBuffer::<P>::new();
        std::thread::spawn(move || {
            let mut incoming_buffer = [0; BUFFER_SIZE];
            info!("Read thread started");
            'read_loop: loop {
                loop {
                    match busy_state_receiver.try_recv() {
                        Ok(busy_state) => protocol.update_busy_state(busy_state),
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            info!("Read thread seems to be disconnected from main thread. Will be shut down.");
                            break 'read_loop;
                        }
                    }
                }
                match client_read.read(&mut incoming_buffer) {
                    Ok(message_length) => {
                        if message_length == 0 {
                            // nothing to do
                        } else {
                            let mut buffer = &incoming_buffer[0..message_length];
                            debug!("New incoming buffer: {:?}", buffer);
                            while let Some((command, message)) = protocol.process_new_buffer(buffer)
                            {
                                buffer = &[];
                                if let Some((command, message)) =
                                    P::message_is_send_via_immediate_route(
                                        &command,
                                        &message,
                                        &protocol.get_busy_state(),
                                    ) {
                                    if let Some(message) = P::construct_message(command, &message) {
                                        if let Err(err) = client_read.write(&message) {
                                            if message_sender
                                                .send(Err(ReadThreadErrors::WriteError(err)))
                                                .is_err()
                                            {
                                                info!("Read thread seems to be disconnected from main thread. Will be shut down.");
                                                break 'read_loop; //disconnected
                                            }
                                        }
                                    } else if message_sender
                                        .send(Err(ReadThreadErrors::ImmediateMessageParseError((
                                            command, message,
                                        ))))
                                        .is_err()
                                    {
                                        info!("Read thread seems to be disconnected from main thread. Will be shut down.");
                                        break 'read_loop; //disconnected
                                    }
                                } else if message_sender.send(Ok((command, message))).is_err() {
                                    info!("Read thread seems to be disconnected from main thread. Will be shut down.");
                                    break 'read_loop; //disconnected
                                }
                            }
                        }
                    }
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::WouldBlock {
                        } else if message_sender
                            .send(Err(ReadThreadErrors::ReadError(err)))
                            .is_err()
                        {
                            break 'read_loop; //disconnected
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_nanos(
                    config.read_iteration_wait_time_ns,
                )); // wait between loops
            }
            info!("Read thread finished");
        });
        Ok(Client {
            busy_state_sender,
            message_receiver,
            stream: client,
        })
    }
    pub fn update_busy_state(&mut self, new_busy_state: P::BusyStates) -> BusyStateUpdateResult {
        match self.busy_state_sender.send(new_busy_state) {
            Ok(()) => BusyStateUpdateResult::Success,
            Err(_) => BusyStateUpdateResult::Disconnected,
        }
    }
    pub fn get_message(&mut self) -> Result<Result<Message<P>, ReadThreadErrors<P>>, TryRecvError> {
        self.message_receiver.try_recv()
    }
    pub fn write_message(
        &mut self,
        command: P::Commands,
        message_: &[u8],
    ) -> Result<(), WriteMessageErrors> {
        let message = P::construct_message(command, message_)
            .ok_or(WriteMessageErrors::MessageConstructionFailed)?;
        let result = self
            .stream
            .write_all(&message)
            .map_err(WriteMessageErrors::MessageSendFailed);
        info!("Message send succesfully:{:?}", (command, message_));
        result
    }
}
