extern crate rustc_serialize;
extern crate kademlia;

use std::io;
use kademlia::*;

fn main() {
    let handle = Kademlia::start("test_net",
                                 Key::random(),
                                 "127.0.0.1:0",
                                 "127.0.0.1:0");

    let k = Key::from(String::from("b75fbc230fb4661d846cfda50b91027e61596c01")).dist(Key::from(String::from("b75fbc230fb4661d846cfda50b91027e61596c00")));
    let l = Key::from(String::from("f75fbc230fb4661d846cfda50b91027e61596c00")).dist(Key::from(String::from("b75fbc230fb4661d846cfda50b91027e61596c00")));
    println!("{:?}", k);
    println!("{:?}", l);
    println!("{:?}", k.cmp(&l));

    let mut input = io::stdin();
    loop {
        let mut buffer = String::new();
        if input.read_line(&mut buffer).is_err() {
            break;
        }
        let args = buffer.trim_right().split(' ').collect::<Vec<_>>();
        match args[0].as_ref() {
            "p" => {
                handle.ping(args[1].as_ref());
            }
            "s" => {
                handle.store(args[1].as_ref(), args[2], args[3]);
            }
            "fn" => {
                handle.find_node(args[1].as_ref(), Key::from(String::from(args[2])));
            }
            "fv" => {
                handle.find_value(args[1].as_ref(), args[2]);
            }
            _ => {
                println!("no match");
            }
        }
    }
}
