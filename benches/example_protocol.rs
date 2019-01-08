use rust_tcp_ipc::{ParseHeaderError, Protocol};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandsExample {
    Start, // = [48, b'0'],
    Funny, // = [b'0', b'0'],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BusyStatesExample {
    Idle,
    Working,
}

const LENGTH_SIZE_EXAMPLE: usize = 3;
const COMMAND_SIZE_EXAMPLE: usize = 2;
const HEADER_SIZE_EXAMPLE: usize = LENGTH_SIZE_EXAMPLE + COMMAND_SIZE_EXAMPLE;
#[derive(Debug)]
pub enum ProtocolExample {}

impl Protocol for ProtocolExample {
    type CommandAsArray = [u8; COMMAND_SIZE_EXAMPLE];
    type HeaderAsArray = [u8; HEADER_SIZE_EXAMPLE];
    type LengthAsArray = [u8; LENGTH_SIZE_EXAMPLE];
    type Commands = CommandsExample;
    type BusyStates = BusyStatesExample;
    fn idle() -> Self::BusyStates {
        BusyStatesExample::Idle
    }
    fn message_is_answered_via_immediate_route(
        _command: &Self::Commands,
        _message: &[u8],
        _busy_state: &Self::BusyStates,
    ) -> Option<(Self::Commands, Vec<u8>)> {
        None
    }
    fn message_slice_to_header_array(input: &[u8]) -> Option<(&Self::HeaderAsArray, &[u8])> {
        if input.len() >= HEADER_SIZE_EXAMPLE {
            Some((
                unsafe {
                    &*(input[0..HEADER_SIZE_EXAMPLE].as_ptr() as *const [_; HEADER_SIZE_EXAMPLE])
                },
                &input[HEADER_SIZE_EXAMPLE..],
            ))
        } else {
            None
        }
    }
    fn parse_command(command: &Self::CommandAsArray) -> Option<Self::Commands> {
        use self::CommandsExample::*;
        match command {
            [b'0', b'0'] => Some(Start),
            [b'4', b'2'] => Some(Funny),
            _ => None,
        }
    }
    fn parse_length(length: &Self::LengthAsArray) -> Option<usize> {
        let length: usize = length
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &x)| (u32::from(x) * 10u32.pow(i as u32)) as usize)
            .sum();
        Some(length)
    }
    fn split_header_array(
        header: &Self::HeaderAsArray,
    ) -> (&Self::CommandAsArray, &Self::LengthAsArray) {
        (
            unsafe {
                &*(header[LENGTH_SIZE_EXAMPLE..HEADER_SIZE_EXAMPLE].as_ptr()
                    as *const [_; COMMAND_SIZE_EXAMPLE])
            },
            unsafe {
                &*(header[0..LENGTH_SIZE_EXAMPLE].as_ptr() as *const [_; LENGTH_SIZE_EXAMPLE])
            },
        )
    }
    fn parse_header(
        header: &Self::HeaderAsArray,
    ) -> Result<(Self::Commands, usize), (ParseHeaderError, &Self::HeaderAsArray)> {
        let (command, length) = Self::split_header_array(header);
        if let Some(command) = Self::parse_command(command) {
            if let Some(length) = Self::parse_length(length) {
                Ok((command, length))
            } else {
                Err((ParseHeaderError::LengthParseFailed, header))
            }
        } else {
            Err((ParseHeaderError::CommandParseFailed, header))
        }
    }
    fn command_to_array(command: Self::Commands) -> Self::CommandAsArray {
        use self::CommandsExample::*;
        match command {
            Start => [b'0', b'0'],
            Funny => [b'4', b'2'],
        }
    }
    fn get_length_as_array(_: Self::Commands, message: &[u8]) -> Option<Self::LengthAsArray> {
        let length = message.len() as u64;
        if length >= 256u64.pow(3) {
            return None;
        }
        let mut length_array = [0; 3];
        for i in 0u32..3 {
            length_array[(2 - i) as usize] = (length / 256u64.pow(i) % 256) as u8;
        }
        Some(length_array)
    }
    fn construct_header(command: Self::CommandAsArray, length: Self::LengthAsArray) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(&length);
        header.extend_from_slice(&command);
        header
    }
}
