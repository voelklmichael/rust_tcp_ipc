use rust_tcp_ipc::*;

mod example_protocol;
use self::example_protocol::*;

use std::net::TcpListener;

use simplelog::*;

fn main() {
    TermLogger::init(LevelFilter::Debug, simplelog::Config::default()).unwrap();

    let server_receiver = start_server();
    std::thread::sleep(std::time::Duration::from_millis(100));
    // start client thread
    let config = ClientConfig {
        connect_wait_time_ms: 5_000,
        read_iteration_wait_time_ns: 1_000,
        shutdown_wait_time_in_ns: 1_000_000,
    };
    let mut client =
        Client::<ProtocolExample>::connect("127.0.0.1:6666", config).expect("client unwrap");

    std::thread::sleep(std::time::Duration::from_micros(500_000));
    assert!(client
        .write_message(CommandsExample::Start, &[0, 2, 3])
        .is_ok());
    assert_eq!(
        client.update_busy_state(BusyStatesExample::Working),
        BusyStateUpdateResult::Success
    );
    assert_eq!(
        client.update_busy_state(BusyStatesExample::Idle),
        BusyStateUpdateResult::Success
    );

    for _ in 0..3 {
        let r = client.get_message();
        println!("{:?}", r);
    }

    client.shutdown().expect("Shutdown failed.");
    std::thread::sleep(std::time::Duration::from_micros(500_000));

    loop {
        use std::sync::mpsc::RecvTimeoutError::*;
        match server_receiver.recv_timeout(std::time::Duration::from_micros(500_000)) {
            Ok(()) => break,
            Err(Timeout) => println!("waiting"),
            Err(Disconnected) => panic!("disconnected"),
        }
    }
    println!("finished");
}

fn start_server() -> std::sync::mpsc::Receiver<()> {
    use std::io::Write;
    // start server thread
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut server, socket_address) = TcpListener::bind("127.0.0.1:6666")
            .unwrap()
            .accept()
            .unwrap();
        println!("server connected to {:?}", socket_address);

        use self::CommandsExample::*;

        println!("---------");
        let message = ProtocolExample::construct_message(Start, &[b'a']).unwrap();
        server.write_all(&message).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        println!("---------");
        let message = ProtocolExample::construct_message(Funny, &[b'a', 0, 1, 2, 3, 4]).unwrap();
        server.write_all(&message).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        println!("---------");
        let message = ProtocolExample::construct_message(Start, &[b'b', 0, 1, 4]).unwrap();
        server.write_all(&message).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        std::thread::sleep(std::time::Duration::from_micros(1_000_000));
        println!("Server closed");
        sender.send(()).expect("Server finished send failed.");
    });
    receiver
}
