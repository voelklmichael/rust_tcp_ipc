mod example;
pub use self::example::*;

use std::fmt::Debug;

#[derive(Debug)]
pub enum ParseHeaderError {
    CommandParseFailed,
    LengthParseFailed,
}
pub trait Protocol {
    type CommandAsArray;
    type HeaderAsArray: Debug;
    type LengthAsArray;
    type Commands: Clone + Copy + Debug + PartialEq;
    type BusyStates: Clone + Copy + Debug + PartialEq;
    fn idle() -> Self::BusyStates;
    fn message_is_send_via_immediate_route(
        command: &Self::Commands,
        message: &Vec<u8>,
        busy_state: &Self::BusyStates,
    ) -> Option<(Self::Commands, Vec<u8>)>;
    fn parse_command(command: &Self::CommandAsArray) -> Option<Self::Commands>;
    fn parse_length(length: &Self::LengthAsArray) -> Option<usize>;
    fn message_slice_to_header_array(input: &[u8]) -> Option<(&Self::HeaderAsArray, &[u8])>;
    fn split_header_array(
        header: &Self::HeaderAsArray,
    ) -> (&Self::CommandAsArray, &Self::LengthAsArray);
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
    fn command_to_array(command: Self::Commands) -> Self::CommandAsArray;
    fn get_length_as_array(command: Self::Commands, message: &[u8]) -> Option<Self::LengthAsArray>;
    fn construct_header(command: Self::CommandAsArray, length: Self::LengthAsArray) -> Vec<u8>;
    fn construct_message(command: Self::Commands, message: &[u8]) -> Option<Vec<u8>> {
        if let Some(length) = Self::get_length_as_array(command, message) {
            let command = Self::command_to_array(command);
            let mut new_message = Self::construct_header(command, length);
            new_message.extend_from_slice(message);
            Some(new_message)
        } else {
            None
        }
    }
}
