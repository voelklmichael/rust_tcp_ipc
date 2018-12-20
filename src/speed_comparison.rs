use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// this function is a minimial speed test
pub fn speed_comparison() {
    std::thread::spawn(move || {
        let mut server = TcpListener::bind("127.0.0.1:42042")
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
    let mut client = TcpStream::connect("127.0.0.1:42042").expect("Unable to connect to server");

    std::thread::sleep(std::time::Duration::from_millis(100));

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
    assert!(seconds == 0);
    let delta = (delta.subsec_nanos()) as f64 / n as f64 / 1e3;
    println!("{:?}μs", delta);
}
