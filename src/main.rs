#![feature(io)]

extern crate rand;

use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::fmt::{Error, Debug, Formatter};

/// Length of an ID, in bytes
const KEY_LEN: usize = 20;
/// Number of buckets (length of ID in bits)
const N_BUCKETS: usize = KEY_LEN * 8;
/// Number of nodes in each bucket
const BUCKET_SIZE: usize = 20;

#[derive(Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
struct Key([u8; KEY_LEN]);

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter().rev() {
            try!(write!(f, "{0:02x}", x));
        }
        Ok(())
    }
}

#[derive(Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
struct Distance([u8; KEY_LEN]);

impl Debug for Distance {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter() {
            try!(write!(f, "{0:02x}", x));
        }
        Ok(())
    }
}

impl Distance {
    fn zeroes_in_prefix(&self) -> usize {
        for i in 0..KEY_LEN {
            for j in 8us..0 {
                if (self.0[i] >> (7 - j)) & 0x1 != 0 {
                    return i * 8 + j;
                }
            }
        }
        KEY_LEN * 8 - 1
    }
}

#[derive(Debug,Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
struct NodeInfo {
    id: Key,
}

struct DHTEndpoint {
    routes: RoutingTable,
    net_id: String,
}

impl DHTEndpoint {
    fn new(node: NodeInfo, net_id: String) -> DHTEndpoint {
        DHTEndpoint {
            routes: RoutingTable::new(node),
            net_id: net_id,
        }
    }
}

struct RoutingTable {
    node: NodeInfo,
    buckets: Vec<Vec<NodeInfo>>
}

impl RoutingTable {
    fn new(node: NodeInfo) -> RoutingTable {
        let mut buckets = Vec::new();
        for _ in 0..N_BUCKETS {
            buckets.push(Vec::new());
        }
        RoutingTable { node: node, buckets: buckets }
    }

    fn update(&mut self, node: NodeInfo) {
        let bucket_index = dist(self.node.id, node.id).zeroes_in_prefix();
        let bucket = &mut self.buckets[bucket_index];
        let node_index = bucket.iter().position(|x| *x == node);
        match node_index {
            Some(i) => {
                let swap = bucket[i];
                bucket[i] = bucket[0];
                bucket[0] = swap;
            },
            None => {
                if bucket.len() < BUCKET_SIZE {
                    bucket.push(node);
                }
            },
        }
    }

    fn find_closest_nodes(&self, item: Key, count: usize) -> Vec<(NodeInfo, Distance)> {
        if count == 0 {
            return Vec::new();
        }
        let bucket_index = dist(self.node.id, item).zeroes_in_prefix();
        let mut ret = Vec::with_capacity(count);
        for i in bucket_index..N_BUCKETS {
            for c in &self.buckets[i] {
                ret.push( (*c, dist(c.id, item)) );
                if ret.len() == count {
                    ret.sort_by(|&(_,a), &(_,b)| a.cmp(&b));
                    return ret;
                }
            }
        }
        if bucket_index == 0 {
            return ret;
        }
        for i in (bucket_index-1)..0 {
            for c in &self.buckets[i] {
                ret.push( (*c, dist(c.id, item)) );
                if ret.len() == count {
                    ret.sort_by(|&(_,a), &(_,b)| a.cmp(&b));
                    return ret;
                }
            }
        }
        ret.sort_by(|&(_,a), &(_,b)| a.cmp(&b));
        ret
    }

}

fn new_random_key() -> Key {
    let mut res = [0; KEY_LEN];
    for i in 0us..KEY_LEN {
        res[i] = rand::random::<u8>();
    }
    Key(res)
}

fn dist(x: Key, y: Key) -> Distance{
    let mut res = [0; KEY_LEN];
    for i in 0us..KEY_LEN {
        res[i] = x.0[i] ^ y.0[i];
    }
    Distance(res)
}

fn handle_client(stream: TcpStream) {
    let mut s = String::new();
    let mut stream = stream;
    stream.read_to_string(&mut s);
    println!("{}", s);
}

fn main() {
    let dht = DHTEndpoint::new(NodeInfo { id: new_random_key() }, "test_net".to_string());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    println!("DHT endpoint setup for network '{}' complete", dht.net_id);
    println!("Server started at {:?}", listener.local_addr().unwrap());
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    handle_client(stream);
                });
            }
            Err(e) => {
                println!("Some connection failed");
            }
        }
    }
}
