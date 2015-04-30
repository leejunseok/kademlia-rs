extern crate rustc_serialize;
extern crate kademlia;

use std::io;
use kademlia::*;

fn main() {
    let handle = Kademlia::start("test_net",
                                 Key::random(),
                                 "127.0.0.1:0",
                                 "127.0.0.1:0");

    let mut dummy_info = NodeInfo {
        net_id: String::from("test_net"),
        addr: String::from("asdfasdf"),
        id: Key::random(),
    };

    let mut input = io::stdin();
    loop {
        let mut buffer = String::new();
        if input.read_line(&mut buffer).is_err() {
            break;
        }
        let args = buffer.trim_right().split(' ').collect::<Vec<_>>();
        match args[0].as_ref() {
            "p" => {
                dummy_info.addr = String::from(args[1]);
                handle.ping(dummy_info.clone());
            }
            "s" => {
                dummy_info.addr = String::from(args[1]);
                handle.store(dummy_info.clone(), args[2], args[3]);
            }
            "fn" => {
                dummy_info.addr = String::from(args[1]);
                handle.find_node(dummy_info.clone(), Key::from(String::from(args[2])));
            }
            "fv" => {
                dummy_info.addr = String::from(args[1]);
                handle.find_value(dummy_info.clone(), args[2]);
            }
            _ => {
                println!("no match");
            }
        }
    }
}
