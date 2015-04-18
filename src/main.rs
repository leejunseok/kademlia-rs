extern crate rand;
extern crate rustc_serialize;

use rustc_serialize::{Decodable, Encodable, Decoder, Encoder};
use rustc_serialize::hex::ToHex;

use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::fmt::{Error, Debug, Formatter};

/// Length of an ID, in bytes
const KEY_LEN: usize = 20;
/// Number of buckets (length of ID in bits)
const N_BUCKETS: usize = KEY_LEN * 8;
/// Number of nodes in each bucket
const BUCKET_SIZE: usize = 20;
/// Message length
const MESSAGE_LEN: usize = 256;

#[derive(Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
struct Key([u8; KEY_LEN]);

impl Decodable for Key {
    fn decode<D: Decoder>(d: &mut D) -> Result<Key, D::Error> {
        let mut ret = [0; KEY_LEN];
        for i in 0..KEY_LEN {
            ret[i] = try!(d.read_u8());
        }
        Ok(Key(ret))
    }
}

impl Encodable for Key {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        for byte in &self.0 {
            s.emit_u8(*byte);
        }
        Ok(())
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

#[derive(Debug,Copy,Clone)]
struct NodeInfo {
    id: Key,
    addr: SocketAddr,
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

    /// Update the appropriate bucket with the new node's info
    fn update(&mut self, node: NodeInfo) {
        let bucket_index = dist(self.node.id, node.id).zeroes_in_prefix();
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
    fn lookup_nodes(&self, item: Key, count: usize) -> Vec<(NodeInfo, Distance)> {
        if count == 0 {
            return Vec::new();
        }
        let mut ret = Vec::with_capacity(count);
        for bucket in &self.buckets {
            for node in bucket {
                ret.push( (*node, dist(node.id, item)) );
            }
        }
        ret.sort_by(|&(_,a), &(_,b)| a.cmp(&b));
        ret.truncate(count);
        return ret;
    }

}

/// Returns a random, KEY_LEN long byte string.
fn new_random_key() -> Key {
    let mut res = [0; KEY_LEN];
    for i in 0us..KEY_LEN {
        res[i] = rand::random::<u8>();
    }
    Key(res)
}

/// XORs two Keys
fn dist(x: Key, y: Key) -> Distance{
    let mut res = [0; KEY_LEN];
    for i in 0us..KEY_LEN {
        res[i] = x.0[i] ^ y.0[i];
    }
    Distance(res)
}


#[derive(RustcEncodable, RustcDecodable)]
enum Request {
    PingRequest,
    StoreRequest,
    FindNodeRequest,
    FindValueRequest,
}

#[derive(RustcEncodable, RustcDecodable)]
enum Response {
    PingResponse,
    StoreResponse,
    FindNodeResponse,
    FindValueResponse,
}

#[derive(RustcEncodable, RustcDecodable)]
enum Payload {
    Request(Request),
    Response(Response),
}

#[derive(RustcEncodable, RustcDecodable)]
struct Message {
    src: Key,
    token: Key,
    payload: Payload,
}


fn handle_client(mut stream: TcpStream) {
    let mut msg = Vec::new();
    for byte in stream.bytes() {
        let byte = byte.unwrap();
        msg.push(byte);
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let dht = DHTEndpoint::new(NodeInfo { id: new_random_key(), addr: addr }, "test_net".to_string());
    println!("DHT endpoint setup for network '{}' complete", dht.net_id);
    println!("Server started at {}", addr);
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
