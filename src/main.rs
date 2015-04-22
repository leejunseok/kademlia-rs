extern crate kademlia;
extern crate rustc_serialize;

use kademlia::*;

fn main() {
    let mut handle = Kademlia::start("test_net",
                                     Key::random(),
                                     "127.0.0.1:0",
                                     "127.0.0.1:0");
    loop {}
}
