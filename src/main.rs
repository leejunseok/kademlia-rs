extern crate dht;
extern crate rustc_serialize;

use dht::*;

fn main() {
    let handle = Kademlia::start("test_net", Key::random(), "127.0.0.1:0", "127.0.0.1:0");
    loop {}
}
