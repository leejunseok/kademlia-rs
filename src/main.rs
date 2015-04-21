extern crate kademlia;
extern crate rustc_serialize;

use kademlia::*;

fn main() {
    let mut nodes = Vec::new();
    for _ in 0..60 {
        nodes.push(Kademlia::start("test_net", Key::random(), "127.0.0.1:0", "127.0.0.1:0"));
    }
    loop {}
}
