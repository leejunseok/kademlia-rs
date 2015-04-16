extern crate rustc_serialize;
extern crate rand;

use rustc_serialize::hex::*;
use std::fmt::{Error, Display, Formatter};

const ID_SIZE: usize = 20;
const N_BUCKETS: usize = ID_SIZE * 8 - 1;
const BUCKET_SIZE: usize = 20;

#[derive(Debug,Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
struct NodeID([u8; ID_SIZE]);

impl NodeID {
    fn zeroes_in_prefix(&self) -> usize {
        for i in 0..ID_SIZE {
            for j in 8us..0 {
                if (self.0[i] >> (7 - j)) & 0x1 != 0 {
                    return i * 8 + j;
                }
            }
        }
        ID_SIZE * 8 - 1
    }
}

#[derive(Debug,Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
struct Contact {
    id: NodeID,
}

struct RoutingTable {
    origin: NodeID,
    buckets: Vec<Vec<Contact>>
}

impl RoutingTable {
    fn new(origin: NodeID) -> RoutingTable{
        let mut buckets = Vec::new();
        for i in 0..N_BUCKETS {
            buckets.push(Vec::new());
        }
        RoutingTable { origin: origin, buckets: buckets }
    }

    fn update(&mut self, contact: Contact) {
        let prefix_len = dist(self.origin, contact.id).zeroes_in_prefix();
        let bucket = &mut self.buckets[prefix_len];
        let index = bucket.iter().position(|x| *x == contact);
        match index {
            Some(i) => {
                let swap = bucket[i];
                bucket[i] = bucket[0];
                bucket[0] = swap;
            },
            None => {
                if bucket.len() < BUCKET_SIZE {
                    bucket.push(contact);
                }
            },
        }
    }
}

fn new_node_id(input: &str) -> Result<NodeID, &str> {
    let mut input = match input.from_hex() {
        Err(_) => { return Err("Input was not valid hex"); }
        Ok(x) => { x }
    };

    input.reverse();

    if ID_SIZE != input.len() {
        return Err("The length of the input doesn't correspond to the ID_SIZE");
    };

    let mut res = [0; ID_SIZE];
    for i in 0us..ID_SIZE {
        res[i] = input[i];
    }
    Ok(NodeID(res))
}

fn new_random_node_id() -> NodeID {
    let mut res = [0; ID_SIZE];
    for i in 0us..ID_SIZE {
        res[i] = rand::random::<u8>();
    }
    NodeID(res)
}

impl Display for NodeID {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter().rev() {
            try!(write!(f, "{0:02x}", x));
        }
        Ok(())
    }
}

fn dist(x: NodeID, y: NodeID) -> NodeID{
    let mut res = [0; ID_SIZE];
    for i in 0us..ID_SIZE {
        res[i] = x.0[i] ^ y.0[i];
    }
    NodeID(res)
}

fn main() {
    let j = new_random_node_id();
    let k = new_random_node_id();
    println!("{}", j);
    println!("{}", k);
    println!("{:?}", j.cmp(&k));
    println!("{:?}", j.cmp(&j));
}
