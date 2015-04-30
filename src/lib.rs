extern crate rand;
extern crate rustc_serialize;
extern crate crypto;

mod kademlia;
mod key;
mod rpc;
mod routing;

pub use kademlia::Kademlia;
pub use key::Key;
pub use routing::NodeInfo;

const K: usize = 20;
const N_BUCKETS: usize = K * 8;
const BUCKET_SIZE: usize = 20;
const MESSAGE_LEN: usize = 8196;
const TIMEOUT: u32 = 5000;
const ALPHA: isize = 3;
