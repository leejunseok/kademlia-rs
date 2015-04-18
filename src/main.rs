extern crate dht;

use dht::*;

use std::io::Read;

fn main() {
    let dht = DHTEndpoint::start(Key::random(), "127.0.0.1:0", "test_net".to_string());
    println!("DHT endpoint setup for network '{}' complete", dht.net_id);
}
