e[![Documentation](https://docs.rs/rust_tcp_ipc/badge.svg)](https://docs.rs/rust_tcp_ipc)
# rust_tcp_ipc
This is a crate for Interprocess Communication via TCP.

It allows for easy, asynchronous sending and receiving messages/commands.

A flexible protocol is used, consisting of a command, a length and a payload.

In detail, it is expected that the used TCP protocol works via exchange of byte collections.
A fixed header length is assumed, so - for example - the first 5 bytes of each message encode the message header.
The header in turn consists of a command (like Stop, Start, Pause, Load, ...) and a length part.
Command & length can be in arbitrary order (but have to be fixed for the protocol).
Then the next length-many bytes which are received are the payload of the message.
Further received bytes form the next message.

An example is given in the Examples.

To work on this crate was motivated by a Talk given at the Regensburg Haskell Meetup in November 2018.
