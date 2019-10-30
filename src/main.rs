extern crate rustc_serialize;
extern crate kademlia;
extern crate env_logger;
extern crate structopt;

use std::io;
use kademlia::*;
use structopt::StructOpt;


#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(long)]
    id: Option<String>,
    
    #[structopt(long, default_value = "127.0.0.1:0")]
    bind: String,

    #[structopt(long)]
    bootstrap_addr: Option<String>,

    #[structopt(long)]
    bootstrap_id: Option<String>,


    #[structopt(long, default_value = "test_net")]
    network_id: String,
}


fn main() {
    env_logger::init();

    let opt = Opt::from_args();
    println!("{:#?}", opt);


    let input = io::stdin();
    let bootstrap = if opt.bootstrap_addr.is_some() && opt.bootstrap_id.is_some() {
        Some(NodeInfo {
            id: Key::from(opt.bootstrap_id.unwrap()),
            addr: opt.bootstrap_addr.unwrap(),
            net_id: opt.network_id.clone(),
        })
    } else {
        None
    };
    let handle = Kademlia::start(opt.network_id.clone(),
                                 match opt.id {
                                    Some(k) => Key::from(k),
                                    None => Key::random()
                                 },
                                 &opt.bind,
                                 bootstrap);

    let mut dummy_info = NodeInfo {
        net_id: opt.network_id,
        addr: String::from("asdfasdf"),
        id: Key::random(),
    };

    loop {
        let mut buffer = String::new();
        if input.read_line(&mut buffer).is_err() {
            break;
        }
        let args = buffer.trim_end().split(' ').collect::<Vec<_>>();
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
            "put" => {
                println!("{:?}", handle.put(String::from(args[1]), String::from(args[2])));
            }
            "get" => {
                println!("{:?}", handle.get(String::from(args[1])));
            }
            _ => {
                println!("no match");
            }
        }
    }
}
