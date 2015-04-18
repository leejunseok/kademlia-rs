extern crate rand;

use std::fmt::{Error, Debug, Formatter};
use std::net::SocketAddr;

const KEY_LEN: usize = 20;
const N_BUCKETS: usize = KEY_LEN * 8;
const BUCKET_SIZE: usize = 20;
const MESSAGE_LEN: usize = 256;

pub struct DHTEndpoint {
    pub routes: RoutingTable,
    pub net_id: String,
}

impl DHTEndpoint {
    pub fn new(node: NodeInfo, net_id: String) -> DHTEndpoint {
        DHTEndpoint {
            routes: RoutingTable::new(node),
            net_id: net_id,
        }
    }
}

pub struct RoutingTable {
    pub node: NodeInfo,
    pub buckets: Vec<Vec<NodeInfo>>
}

impl RoutingTable {
    pub fn new(node: NodeInfo) -> RoutingTable {
        let mut buckets = Vec::new();
        for _ in 0..N_BUCKETS {
            buckets.push(Vec::new());
        }
        RoutingTable { node: node, buckets: buckets }
    }

    /// Update the appropriate bucket with the new node's info
    pub fn update(&mut self, node: NodeInfo) {
        let bucket_index = Distance::dist(self.node.id, node.id).zeroes_in_prefix();
        let bucket = &mut self.buckets[bucket_index];
        let node_index = bucket.iter().position(|x| x.id == node.id);
        match node_index {
            Some(i) => {
                let temp = bucket.remove(i);
                bucket.push(temp);
            }
            None => {
                if bucket.len() < BUCKET_SIZE {
                    bucket.push(node);
                } else {
                    // go through bucket, pinging nodes, replace one
                    // that doesn't respond.
                }
            }
        }
    }

    /// Lookup the nodes closest to item in this table
    ///
    /// NOTE: This method is a really stupid, linear time search. I can't find
    /// info on how to use the buckets effectively to solve this.
    pub fn lookup_nodes(&self, item: Key, count: usize) -> Vec<(NodeInfo, Distance)> {
        if count == 0 {
            return Vec::new();
        }
        let mut ret = Vec::with_capacity(count);
        for bucket in &self.buckets {
            for node in bucket {
                ret.push( (*node, Distance::dist(node.id, item)) );
            }
        }
        ret.sort_by(|&(_,a), &(_,b)| a.cmp(&b));
        ret.truncate(count);
        return ret;
    }

}

#[derive(Debug,Copy,Clone)]
pub struct NodeInfo {
    pub id: Key,
    pub addr: SocketAddr,
}

#[derive(Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
pub struct Key([u8; KEY_LEN]);

impl Key {
    /// Returns a random, KEY_LEN long byte string.
    pub fn random() -> Key {
        let mut res = [0; KEY_LEN];
        for i in 0us..KEY_LEN {
            res[i] = rand::random::<u8>();
        }
        Key(res)
    }
}

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter().rev() {
            try!(write!(f, "{0:02x}", x));
        }
        Ok(())
    }
}

#[derive(Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
pub struct Distance([u8; KEY_LEN]);

impl Distance {
    /// XORs two Keys
    pub fn dist(x: Key, y: Key) -> Distance{
        let mut res = [0; KEY_LEN];
        for i in 0us..KEY_LEN {
            res[i] = x.0[i] ^ y.0[i];
        }
        Distance(res)
    }

    pub fn zeroes_in_prefix(&self) -> usize {
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

impl Debug for Distance {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter() {
            try!(write!(f, "{0:02x}", x));
        }
        Ok(())
    }
}

struct Message {
    src: Key,
    token: Key,
    payload: Payload,
}

enum Payload {
    Request(Request),
    Response(Response),
}

enum Request {
    PingRequest,
    StoreRequest,
    FindNodeRequest,
    FindValueRequest,
}

enum Response {
    PingResponse,
    StoreResponse,
    FindNodeResponse,
    FindValueResponse,
}
