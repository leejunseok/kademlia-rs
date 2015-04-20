extern crate dht;
extern crate rustc_serialize;

use dht::*;

fn main() {
    let ni = NodeInfo { id: Key::random(), addr: String::from("127.0.0.1:0") };
    let msg = Message { src: ni, token: Key::random(), payload: Payload::Request(Request::FindNodeRequest(Key::random()))};
    println!("{}", rustc_serialize::json::encode(&msg).unwrap());
    println!("{}", rustc_serialize::json::encode(&msg).unwrap().len());
    let mut handle = DhtNode::new("test_net", Key::random(), "127.0.0.1:0");
    DhtHandle::start(handle, "127.0.0.1:0");
    loop {}
}
