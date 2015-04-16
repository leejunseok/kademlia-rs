extern crate rustc_serialize;
extern crate rand;

use rustc_serialize::hex::*;
use std::fmt::{Error, Display, Formatter};

const ID_LEN: usize = 20;

#[derive(Debug)]
struct NodeID([u8; ID_LEN]);

fn main() {
    let k = new_node_id("00000000000000000000000000000000000000ff").unwrap();
    let l = new_random_node_id();
    println!("{}", k);
    println!("{}", l);
}

fn new_node_id(input: &str) -> Result<NodeID, &str> {
    let mut input = match input.from_hex() {
        Err(_) => { return Err("Input was not valid hex"); }
        Ok(x) => { x }
    };

    input.reverse();

    if ID_LEN != input.len() {
        return Err("The length of the input doesn't correspond to the ID_LEN");
    };

    let mut res = [0; ID_LEN];
    for i in 0us..ID_LEN {
        res[i] = input[i];
    }
    Ok(NodeID(res))
}

fn new_random_node_id() -> NodeID {
    let mut res = [0; ID_LEN];
    for i in 0us..ID_LEN {
        res[i] = rand::random::<u8>();
    }
    NodeID(res)
}

impl Display for NodeID {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter().rev() {
            write!(f, "{0:02x}", x);
        }
        Ok(())
    }
}
