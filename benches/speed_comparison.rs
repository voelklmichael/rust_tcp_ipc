mod example_protocol;
//extern crate criterion;
use criterion::*;

// this function is a minimial speed test for the tcp protocol
// this is the source for the definition of constant "TIME_LIMIT_IN_US"
fn speed_check_tcp_standard(c: &mut criterion::Criterion) {
    use std::io::{Read, Write};
    std::thread::spawn(move || {
        let mut server = std::net::TcpListener::bind("127.0.0.1:42042")
            .unwrap()
            .accept()
            .expect("Unable to start server")
            .0;
        server.set_nodelay(true).expect("Failed to set 'NoDelay'");
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
    client.set_nodelay(true).expect("Failed to set 'NoDelay'");

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

    c.bench_function("speed_check_tcp_standard", |b| {
        b.iter(|| {
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
        })
    });
}

// this is a version of the previous test, but using mio (much faster! - ~8μs instead of ~18μs)
// so the crate uses mio for the moment
fn speed_check_tcp_mio(c: &mut criterion::Criterion) {
    use mio::net::{TcpListener, TcpStream};
    use std::io::{Read, Write};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let size = 128;

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4242);
    std::thread::spawn(move || {
        let server = TcpListener::bind(&socket).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let mut server = loop {
            match server.accept() {
                Ok(server) => break server.0,
                Err(err) => match err.kind() {
                    std::io::ErrorKind::WouldBlock => continue,
                    _ => panic!("server error: {:?}", err),
                },
            }
        };
        std::thread::sleep(std::time::Duration::from_millis(100));
        server.set_nodelay(true).unwrap();
        server.set_send_buffer_size(size).unwrap();
        server.set_recv_buffer_size(size).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let mut buffer = [0; 128];
        loop {
            let n = match server.read(&mut buffer) {
                Ok(n) => n,
                Err(err) => match err.kind() {
                    std::io::ErrorKind::WouldBlock => continue,
                    x => panic!("server receive error: {:?}", x),
                },
            };
            assert_eq!(n, 16);
            if n == 0 {
                panic!("Server Message receiving: no message");
            }
            server
                .write(&buffer[0..n])
                .expect("Server failed to write message");
        }
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut client = TcpStream::connect(&socket).expect("Unable to connect to server");

    client.set_nodelay(true).unwrap();
    client.set_send_buffer_size(size).unwrap();
    client.set_recv_buffer_size(size).unwrap();

    loop {
        match client.write(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]) {
            Ok(n) => {
                assert_eq!(n, 16);
                break;
            }
            Err(err) => match err.kind() {
                std::io::ErrorKind::WouldBlock => continue,
                x => panic!("client write error: {:?}", x),
            },
        };
    }
    let mut buffer = [0; 128];
    let n = loop {
        match client.read(&mut buffer) {
            Ok(n) => break n,
            Err(err) => match err.kind() {
                std::io::ErrorKind::WouldBlock => continue,
                x => panic!("client receive error: {:?}", x),
            },
        }
    };
    if n == 0 {
        panic!("Client Message receiving: no message");
    }
    assert_eq!(n, 16);

    // start iterations

    c.bench_function("speed_check_tcp_mio", |b| {
        b.iter(|| {
            loop {
                match client.write(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]) {
                    Ok(n) => {
                        assert_eq!(n, 16);
                        break;
                    }
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::WouldBlock => continue,
                        x => panic!("client write error: {:?}", x),
                    },
                };
            }
            let n = loop {
                match client.read(&mut buffer) {
                    Ok(n) => break n,
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::WouldBlock => continue,
                        x => panic!("client receive error: {:?}", x),
                    },
                }
            };
            if n == 0 {
                panic!("Client Message receiving: no message");
            }
            assert_eq!(n, 16);
        })
    });
}

// this is a speed check of an examplary implementation
fn speed_check_rust_tcp_ipc(c: &mut criterion::Criterion) {
    use crate::example_protocol::*;
    use rust_tcp_ipc::*;

    let config = TcpIpcConfig {
        after_connect_wait_time: Some(std::time::Duration::from_micros(5_000)),
        read_iteration_wait_time: None, //Some(std::time::Duration::from_nanos(500)), //None,
        shutdown_wait_time: Some(std::time::Duration::from_micros(5_000_000)),
        check_count: 10_000,
    };

    std::thread::spawn(move || {
        let mut server = TcpIpc::<ProtocolExample>::server("127.0.0.1:42457", config)
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
        "127.0.0.1:42457",
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

    // starting iterations
    c.bench_function("speed_check_rust_tcp_ipc", |b| {
        b.iter(|| {
            client
                .write_message(CommandsExample::Start, &[0, 2, 3])
                .expect("Client failed to write message");
            let (_, _) = client
                .await_message(std::time::Duration::from_secs(1), None)
                .expect("Client failed to receive message")
                .expect("Await time exceeded");
        });
    });
}

criterion_group!(
    benches,
    //speed_check_tcp_standard,
    speed_check_tcp_mio //speed_check_rust_tcp_ipc
);
criterion_main!(benches);
