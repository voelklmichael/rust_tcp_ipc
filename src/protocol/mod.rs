use std::fmt::Debug;

pub type Message<P> = (<P as Protocol>::Commands, Vec<u8>);

/// The error type for parsing a header which was transferred via TCP.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParseHeaderError {
    CommandParseFailed,
    LengthParseFailed,
}
/// This trait represents the TCP-Protocol to be used.
///
/// Messages are assumed to be given as u8-slice, consisting of a header and a payload.
///
/// The header combines a command (like Start, Stop, Pause, ...) and the lenght of the payload.
pub trait Protocol: 'static {
    /// This type models the possible commands, like Start, Stop, Pause. It typical is represented by an enum.
    /// # Example
    /// ```
    /// enum ExampleCommands {Start, Stop, Pause}
    /// ```
    type Commands: Clone + Copy + Debug + PartialEq + Send + Sync + 'static;
    /// This type models possible busy_states like Idle, Working, Failure.
    /// # Example
    /// ```
    /// enum ExampleBusyStates {Idle, Working, Failure}
    /// ```
    type BusyStates: Clone + Copy + Debug + PartialEq + Send + 'static;
    /// This type represents the commands' underlying u8-array. (Currently, Rust supports no integer generics.)
    /// # Example
    /// ```
    /// type CommandAsArray = [u8;3];
    /// ```
    type CommandAsArray: Debug;
    /// This type represents the payload-length' underlying u8-array. (Currently, Rust supports no integer generics.)
    /// # Example
    /// ```
    /// type LengthAsArray = [u8;2];
    /// ```
    type LengthAsArray: Debug;
    /// This type represents the header' underlying u8-array.
    /// The array size is usually the sum of the command-array size & the length-array size.
    /// (Currently, Rust supports no integer generics.)
    /// # Example
    /// ```
    /// type HeaderAsArray = [u8;5];
    /// ```
    type HeaderAsArray: Debug;
    /// This function returns a default BusyState "Idle".
    /// # Example
    /// ```
    /// fn idle() -> Self::BusyStates {ExampleBusyStates::Idle}
    /// ```
    fn idle() -> Self::BusyStates;
    fn message_is_send_via_immediate_route(
        command: &Self::Commands,
        message: &[u8],
        busy_state: &Self::BusyStates,
    ) -> Option<(Self::Commands, Vec<u8>)>;
    /// This function parses a command-array into a command (enum-variant). If this fails, None is return.
    /// # Example
    /// ```
    /// fn parse_command(command: &Self::CommandAsArray) -> Option<Self::Commands> {
    ///     use self::ExampleCommands::*;
    ///     match command {
    ///         [0,0,0]=>Start,
    ///         [1,2,3]=>Stop,
    ///         [255,255,255]=>Failure,
    ///         _ => None,
    ///     }
    /// }
    /// ```
    fn parse_command(command: &Self::CommandAsArray) -> Option<Self::Commands>;
    /// This function parses a length-array into a payload-length. If this fails, None is return.
    /// It is to be used only internally.
    /// # Example
    /// ```
    /// fn parse_length(length: &Self::LengthAsArray) -> Option<Self::usize> {
    ///     length[0] as usize +length[1] as usize * 256
    /// }
    /// ```
    fn parse_length(length: &Self::LengthAsArray) -> Option<usize>;
    /// This function splits an incoming message into header-array & payload-slice. If this fails (because the message is too short), None is returned.
    /// It is to be used only internally.
    /// # Example
    /// ```
    /// fn message_slice_to_header_array(input: &[u8]) -> Option<(&Self::HeaderAsArray, &[u8])> {
    ///     const HEADER_SIZE_EXAMPLE:usize = 5;
    ///     if input.len() >= HEADER_SIZE_EXAMPLE {
    ///         Some((
    ///             unsafe {
    ///             &*(input[0..HEADER_SIZE_EXAMPLE].as_ptr() as *const [_; HEADER_SIZE_EXAMPLE])
    ///         },
    ///         &input[HEADER_SIZE_EXAMPLE..],
    ///        ))
    ///     } else {
    ///         None
    ///     }
    /// }
    /// ```
    fn message_slice_to_header_array(input: &[u8]) -> Option<(&Self::HeaderAsArray, &[u8])>;
    /// This function splits header-array into a command-array and a length-array.
    /// It is to be used only internally.
    /// # Example
    /// The following example is "length first", so the payload length takes the first (two) bytes from the incoming header. The remaining bytes encode the command.
    /// ```
    /// fn split_header_array(header: &Self::HeaderAsArray) -> (&Self::CommandAsArray, &Self::LengthAsArray) {
    ///     const LENGTH_SIZE_EXAMPLE : usize = 2;
    ///     const HEADER_SIZE_EXAMPLE : usize = 5;
    ///     (
    ///         unsafe {
    ///             &*(header[LENGTH_SIZE_EXAMPLE..HEADER_SIZE_EXAMPLE].as_ptr()
    ///                     as *const [_; COMMAND_SIZE_EXAMPLE])
    ///         },
    ///         unsafe {
    ///             &*(header[0..LENGTH_SIZE_EXAMPLE].as_ptr() as *const [_; LENGTH_SIZE_EXAMPLE])
    ///         },
    ///     )
    /// }
    /// ```
    fn split_header_array(
        header: &Self::HeaderAsArray,
    ) -> (&Self::CommandAsArray, &Self::LengthAsArray);
    fn command_to_array(command: Self::Commands) -> Self::CommandAsArray;
    fn get_length_as_array(command: Self::Commands, message: &[u8]) -> Option<Self::LengthAsArray>;
    fn construct_header(command: Self::CommandAsArray, length: Self::LengthAsArray) -> Vec<u8>;

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
