extern crate dht;
extern crate rustc_serialize;

use dht::*;

fn main() {
    let handle = DhtNode::new("test_net", Key::random(), "127.0.0.1:0");
    handle.start("127.0.0.1:0");
    loop {}
}
