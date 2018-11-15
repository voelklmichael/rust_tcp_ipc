pub use super::protocol::*;
pub use log::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolBuffer<P: Protocol> {
    current_command: Option<P::Commands>,
    current_target: usize,
    current_message: Vec<u8>,
    incoming_buffer_vec: Vec<u8>,
    busy_state: P::BusyStates,
}
impl<P: Protocol> ProtocolBuffer<P> {
    pub fn new() -> Self {
        Self {
            current_command: None,
            current_target: 0,
            current_message: Vec::new(),
            incoming_buffer_vec: Vec::new(),
            busy_state: P::idle(),
        }
    }
    pub fn process_new_buffer(&mut self, incoming_buffer: &[u8]) -> Option<(P::Commands, Vec<u8>)> {
        self.incoming_buffer_vec.extend_from_slice(incoming_buffer);
        if let Some(command) = self.current_command {
            if self.incoming_buffer_vec.len() + self.current_message.len() < self.current_target {
                self.current_message.append(&mut self.incoming_buffer_vec);
                None
            } else {
                let mut completed_message = self.current_message.split_off(0);
                let mut to_append = self
                    .incoming_buffer_vec
                    .split_off(self.current_target - completed_message.len());
                completed_message.append(&mut self.incoming_buffer_vec);
                info!(
                    "Message received: {:?}",
                    (command, completed_message.clone())
                );
                self.incoming_buffer_vec.append(&mut to_append);
                self.current_target = 0; //not strictly necessary
                self.current_command = None;
                Some((command, completed_message))
            }
        } else if let Some((header, message)) =
            P::message_slice_to_header_array(self.incoming_buffer_vec.as_slice())
        {
            let (command, length) = match P::parse_header(header) {
                Ok((command, length)) => (command, length),
                Err((err, message)) => {
                    // this should happen only in two cases:
                    // a) the command is not-known
                    // b) the length of the message is too large
                    // Since both cases should never happen, a panic seems reasonable
                    error!("parse error: {:?}, incoming header: {:?}", err, message);
                    panic!("parse error: {:?}\r\n, incoming header: {:?}", err, message)
                }
            };
            self.current_command = Some(command);
            self.current_target = length;
            self.current_message = message.to_vec(); // capicity can also be set already
            self.incoming_buffer_vec = self.current_message.split_off(length);
            self.current_message
                .reserve(length - self.current_message.len());
            debug!(
                "New message started: {:?}",
                (command, self.current_message.clone())
            );
            self.process_new_buffer(&[]) // process remaining buffer
        } else {
            None
        }
    }
    pub fn update_busy_state(&mut self, busy_state: P::BusyStates) {
        self.busy_state = busy_state;
    }
    pub fn get_busy_state(&self) -> P::BusyStates {
        self.busy_state
    }
}
