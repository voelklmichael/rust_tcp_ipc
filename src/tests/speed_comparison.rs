const _TIME_LIMIT_IN_US: f64 = 25.0; // this is the maximal allowed average response time in microseconds - longer times will fail this tests

#[test]
// this function is a minimial speed test for the tcp protocol
// this is the source for the definition of constant "TIME_LIMIT_IN_US"
fn speed_check_tcp_minimal() {
    use std::io::{Read, Write};
    std::thread::spawn(move || {
        let mut server = std::net::TcpListener::bind("127.0.0.1:42042")
            .unwrap()
            .accept()
            .expect("Unable to start server")
            .0;
        std::thread::sleep(std::time::Duration::from_millis(100));
        loop {
            let mut buffer = [0; 128];
            let n = server
                .read(&mut buffer)
                .expect("Server failed to receive message");
            if n == 0 {
                panic!("Server Message receiving: no message");
            }
            server
                .write(&buffer[0..n])
                .expect("Server failed to write message");
        }
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut client =
        std::net::TcpStream::connect("127.0.0.1:42042").expect("Unable to connect to server");

    std::thread::sleep(std::time::Duration::from_millis(100));

    // send one message to ensure that everything is online
    client
        .write(&[1, 2, 3])
        .expect("Client failed to write message");
    let mut buffer = [0; 128];
    let n = client
        .read(&mut buffer)
        .expect("Client failed to receive message");
    if n == 0 {
        panic!("Client Message receiving: no message");
    }
    println!("starting loop of iterations");

    let now = std::time::Instant::now();
    let n = 10_000;
    for _ in 0..n {
        client
            .write(&[1, 2, 3])
            .expect("Client failed to write message");
        let mut buffer = [0; 128];
        let n = client
            .read(&mut buffer)
            .expect("Client failed to receive message");
        if n == 0 {
            panic!("Client Message receiving: no message");
        }
    }
    let delta = now.elapsed();
    let seconds = delta.as_secs();
    let delta = (seconds as f64 + delta.subsec_nanos() as f64) / n as f64 / 1e3;
    println!("Average iteration time: {:?}μs", delta);
    println!("TIME_LIMIT_IN_US: {:?}μs", _TIME_LIMIT_IN_US);

    assert!(delta < _TIME_LIMIT_IN_US);
}

#[test]
// this is a speed test for an example protocol implementation
// the TCP speed is basically independent of the used implementation
// the bootleneck should be the tcp/ip-os-stuff
fn speed_check() {
    use super::example_protocol::*;
    use crate::*;

    let config = TcpIpcConfig {
        after_connect_wait_time: Some(std::time::Duration::from_micros(5_000)),
        read_iteration_wait_time: None,
        shutdown_wait_time: Some(std::time::Duration::from_micros(5_000_000)),
        check_count: 1,
    };

    std::thread::spawn(move || {
        let mut server = TcpIpc::<ProtocolExample>::server("127.0.0.1:42043", config)
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
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut client = TcpIpc::<ProtocolExample>::client(
        "127.0.0.1:42043",
        config,
        Some(std::time::Duration::from_millis(1)),
    )
    .expect("Unable to connect to server");

    std::thread::sleep(std::time::Duration::from_millis(100));

    // send one message to ensure that everything is online
    client
        .write_message(CommandsExample::Start, &[0, 2, 3])
        .expect("Client failed to write message");
    let (_, _) = client
        .await_message(std::time::Duration::from_secs(1), None)
        .expect("Client failed to receive message")
        .expect("Await time exceeded");

    println!("starting loop of iterations");

    let now = std::time::Instant::now();
    let n = 10_000;
    for _ in 0..n {
        client
            .write_message(CommandsExample::Start, &[0, 2, 3])
            .expect("Client failed to write message");
        let (_, _) = client
            .await_message(std::time::Duration::from_secs(1), None)
            .expect("Client failed to receive message")
            .expect("Await time exceeded");
    }
    let delta = now.elapsed();
    let seconds = delta.as_secs();
    let delta = (seconds as f64 + delta.subsec_nanos() as f64) / n as f64 / 1e3;
    println!("Average iteration time: {:?}μs", delta);
    println!("TIME_LIMIT_IN_US: {:?}μs", _TIME_LIMIT_IN_US);

    assert!(delta < _TIME_LIMIT_IN_US);
}
