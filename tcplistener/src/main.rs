//run the server first
use core::str;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
};

fn handle_client(stream: &mut TcpStream) {
    let buf = &mut [0; 128];
    let nbr_bytes = stream.read(buf).unwrap_or(0);
    println!("{:?}", str::from_utf8(&buf[0..nbr_bytes]));
}

fn main() -> std::io::Result<()> {
    // listen to the laptop
    let listener = TcpListener::bind("192.168.50.67:1234")?;
    dbg!(&listener);
    // accept connections and process them serially
    for stream in listener.incoming() {
        handle_client(&mut stream?);
    }
    Ok(())
}
