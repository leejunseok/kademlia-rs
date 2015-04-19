extern crate dht;
extern crate rustc_serialize;

use dht::*;

fn main() {
    let ni = NodeInfo { id: Key::random(), addr: "127.0.0.1:0".to_string() };
    let msg = Message { src: ni, token: Key::random(), payload: Payload::Request(Request::FindNodeRequest(Key::random()))};
    println!("{}", rustc_serialize::json::encode(&msg).unwrap());
    println!("{}", rustc_serialize::json::encode(&msg).unwrap().len());
    let mut dht = DHTEndpoint::new("test_net".to_string(), Key::random(), "127.0.0.1:0".to_string());
    dht.start("127.0.0.1:0".to_string());
}
