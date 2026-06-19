use std::io::{Read, Write};
use std::net::TcpListener;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9091")?;
    println!("echo server listening on 127.0.0.1:9091");

    if let Ok((mut stream, _)) = listener.accept() {
        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf)?;
        stream.write_all(&buf[..n])?;
    }

    Ok(())
}
