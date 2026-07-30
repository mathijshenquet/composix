use std::io::{Read, Write};
use std::net::TcpListener;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 18084))?;
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request)?;
        let body = common::greeting("proj1-api");
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )?;
    }
    Ok(())
}
