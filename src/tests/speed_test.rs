#[test]
fn main() {
    use super::example_protocol::*;
    use crate::*;

    use simplelog::*;
    //TermLogger::init(LevelFilter::Debug, simplelog::Config::default()).unwrap();

    let config = TcpIpcConfig {
        after_connect_wait_time: Some(std::time::Duration::from_micros(5_000)),
        read_iteration_wait_time: None,
        shutdown_wait_time: Some(std::time::Duration::from_micros(5_000_000)),
    };

    std::thread::spawn(move || {
        let mut server = TcpIpc::<ProtocolExample>::server("127.0.0.1:42042", config)
            .expect("Unable to start server");
        loop {
            let (command, message) = server
                .await_message(std::time::Duration::from_secs(1), None)
                .expect("Server failed to receive message")
                .expect("Await time exceeded");
            server
                .write_message(command, &message)
                .expect("Server failed to write message");
        }
        println!("{:?}", "server thread finished");
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut client = TcpIpc::<ProtocolExample>::client(
        "127.0.0.1:42042",
        config,
        Some(std::time::Duration::from_millis(1)),
    )
    .expect("Unable to connect to server");

    std::thread::sleep(std::time::Duration::from_millis(100));

    let now = std::time::Instant::now();
    let n = 10; //1_000;
    for _ in 0..n {
        client
            .write_message(CommandsExample::Start, &[0, 2, 3])
            .expect("Client failed to write message");
        let (command, message) = client
            .await_message(std::time::Duration::from_secs(1), None)
            .expect("Client failed to receive message")
            .expect("Await time exceeded");
    }
    let delta = now.elapsed();
    let seconds = delta.as_secs();
    assert!(seconds == 0);
    let delta = (delta.subsec_nanos()) as f64 / n as f64 / 1e3;
    println!("{:?}μs", delta);

    panic!("uimpl")
}
