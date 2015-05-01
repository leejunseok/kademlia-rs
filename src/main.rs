extern crate rustc_serialize;
extern crate kademlia;

use std::io;
use kademlia::*;

fn main() {
    let handle = Kademlia::start(String::from("test_net"),
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
                dummy_info.id = Key::from(String::from(args[2]));
                println!("{:?}", handle.ping(dummy_info.clone()));
            }
            "s" => {
                dummy_info.addr = String::from(args[1]);
                dummy_info.id = Key::from(String::from(args[2]));
                println!("{:?}", handle.store(dummy_info.clone(), String::from(args[3]), String::from(args[4])));
            }
            "fn" => {
                dummy_info.addr = String::from(args[1]);
                dummy_info.id = Key::from(String::from(args[2]));
                println!("{:?}", handle.find_node(dummy_info.clone(), Key::from(String::from(args[3]))));
            }
            "fv" => {
                dummy_info.addr = String::from(args[1]);
                dummy_info.id = Key::from(String::from(args[2]));
                println!("{:?}", handle.find_value(dummy_info.clone(), String::from(args[3])));
            }
            "ln" => {
                println!("{:?}", handle.lookup_nodes(Key::from(String::from(args[1]))));
            }
            "lv" => {
                println!("{:?}", handle.lookup_value(String::from(args[1])));
            }
            _ => {
                println!("no match");
            }
        }
    }
}
