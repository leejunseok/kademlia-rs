extern crate dht;

use dht::*;

use std::io::Read;
use std::net::{TcpListener, TcpStream};

fn handle_client(stream: TcpStream) {
    let mut msg = Vec::new();
    for byte in stream.bytes() {
        let byte = byte.unwrap();
        msg.push(byte);
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let dht = DHTEndpoint::new(Key::random(), addr, "test_net".to_string());
    println!("DHT endpoint setup for network '{}' complete", dht.net_id);
    println!("Server started at {}", addr);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    handle_client(stream);
                });
            }
            Err(_) => {
                println!("Some connection failed");
            }
        }
    }
}
