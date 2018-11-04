pub use super::protocol_buffer::*;

pub use std::io::{Read, Write};
pub use std::net::{TcpListener, TcpStream, ToSocketAddrs};
pub use std::sync::mpsc::TryRecvError;

const BUFFER_SIZE: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClientConfig {
    pub connect_wait_time_ms: u64,
    pub read_iteration_wait_time_ns: u64,
    pub shutdown_wait_time_in_ns: u64,
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
    SetNonblockingError,
}
pub struct Client<P: Protocol> {
    busy_state_sender: std::sync::mpsc::Sender<P::BusyStates>,
    message_receiver: std::sync::mpsc::Receiver<Result<Message<P>, ReadThreadErrors<P>>>,
    stream: TcpStream,
    shutdown_sender: std::sync::mpsc::Sender<()>,
    shutdown_wait_time_in_ns: u64,
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
        config: ClientConfig,
    ) -> Result<Client<P>, ConnectErrors> {
        // connect
        let client = {
            let mut error = self::ConnectErrors::SocketListIsEmpty;
            let mut socket_addresses = socket_addresses
                .to_socket_addrs()
                .map_err(ConnectErrors::SocketListParseError)?;
            loop {
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
            }
        };
        // set non-blocking
        if client.set_nonblocking(true).is_err() {
            return Err(self::ConnectErrors::SetNonblockingError);
        }
        // start read thread
        let mut client_read = client.try_clone().map_err(ConnectErrors::TryCloneError)?;
        let (message_sender, message_receiver) = std::sync::mpsc::channel();
        let (busy_state_sender, busy_state_receiver) = std::sync::mpsc::channel();
        let (shutdown_sender, shutdown_receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut protocol = ProtocolBuffer::<P>::new();
            let mut incoming_buffer = [0; BUFFER_SIZE];
            info!("Read thread started");
            'read_loop: loop {
                match shutdown_receiver.try_recv() {
                    Ok(()) => break 'read_loop,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // nothing to do
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        info!("Read thread seems to be disconnected from main thread. Will be shut down.");
                        break 'read_loop;
                    }
                }
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
            shutdown_sender,
            busy_state_sender,
            message_receiver,
            stream: client,
            shutdown_wait_time_in_ns: config.shutdown_wait_time_in_ns,
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
    /// Attemps to close the TCP-connection
    pub fn shutdown(self) -> Result<(), ShutdownError> {
        let shutdown_send_succesfully = match self.shutdown_sender.send(()) {
            Ok(()) => {
                debug!("Shutdown send successfully.");
                true
            }
            Err(_) => {
                warn!("Send of shutdown failed.");
                false
            }
        };

        std::thread::sleep(std::time::Duration::from_nanos(
            self.shutdown_wait_time_in_ns,
        ));

        let shutdown_succesfully = match self.stream.shutdown(std::net::Shutdown::Both) {
            Ok(()) => {
                debug!("Shutdown successfully.");
                true
            }
            Err(_) => {
                warn!("Shutdown failed.");
                false
            }
        };
        if !shutdown_succesfully || !shutdown_succesfully {
            Err(ShutdownError {
                shutdown_succesfully,
                shutdown_send_succesfully,
            })
        } else {
            Ok(())
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShutdownError {
    pub shutdown_send_succesfully: bool,
    pub shutdown_succesfully: bool,
}
