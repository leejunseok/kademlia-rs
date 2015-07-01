#![feature(collections,slice_position_elem)]

#[macro_use]
extern crate log;
extern crate crypto;
extern crate rand;
extern crate rustc_serialize;

mod kademlia;
mod key;
mod rpc;
mod routing;

pub use kademlia::Kademlia;
pub use key::Key;
pub use routing::NodeInfo;

const KEY_LEN: usize = 20;
const N_BUCKETS: usize = KEY_LEN * 8;
const K_PARAM: usize = 8;
const A_PARAM: usize = 3;
const MESSAGE_LEN: usize = 8196;
const TIMEOUT: u32 = 5000;
