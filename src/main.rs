use simplelog::*;

mod client;
mod protocol;
mod protocol_buffer;
use self::client::*;

const BUFFER_SIZE: usize = 512;

fn main() {
    TermLogger::init(LevelFilter::Debug, simplelog::Config::default()).unwrap();

    start_server();
    std::thread::sleep(std::time::Duration::from_millis(100));
    // start client thread
    let config = client::Config {
        connect_wait_time_ms: 5_000,
        read_iteration_wait_time_ns: 1_000,
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
    std::thread::sleep(std::time::Duration::from_micros(500_000));
}

fn start_server() {
    // start server thread
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
    });
    std::thread::sleep(std::time::Duration::from_micros(10));
}
