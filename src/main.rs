extern crate dht;
extern crate rustc_serialize;

use dht::*;

fn main() {
    let mut dht = DHTEndpoint::new(Key::random(), "127.0.0.1:0".to_string(), "test_net".to_string());
    println!("DHT endpoint setup for network '{}' complete", dht.net_id);
    let ni = NodeInfo { id: Key::random(), addr: "127.0.0.1:0".to_string() };
    let msg = Message { src: ni, token: Key::random(), payload: Payload::Request(Request::PingRequest)};
    println!("{}", rustc_serialize::json::encode(&msg).unwrap());
    println!("{}", rustc_serialize::json::encode(&msg).unwrap().len());
    dht.start("127.0.0.1:0".parse().unwrap());
}
