extern crate rand;
extern crate rustc_serialize;

use rustc_serialize::{Decodable, Encodable, Decoder, Encoder};
use rustc_serialize::hex::ToHex;

use std::fmt::{Error, Debug, Formatter};
use std::net::{UdpSocket, SocketAddr};

const KEY_LEN: usize = 20;
const N_BUCKETS: usize = KEY_LEN * 8;
const BUCKET_SIZE: usize = 20;
const MESSAGE_LEN: usize = 8196;

pub struct DHTEndpoint {
    routes: RoutingTable,
    pub net_id: String,
}

impl DHTEndpoint {
    pub fn new(node_id: Key, node_addr: String, net_id: String) -> DHTEndpoint {
        DHTEndpoint {
            routes: RoutingTable::new( NodeInfo { id: node_id, addr: node_addr } ),
            net_id: net_id,
        }
    }

    pub fn start(&mut self, bootstrap: String) {
        let mut socket = UdpSocket::bind(&self.routes.node.addr[..]).unwrap();
        println!("{:?}", socket.local_addr().unwrap());
        let mut buf = [0u8; MESSAGE_LEN];
        loop {
            let (len, src) = socket.recv_from(&mut buf).unwrap();
            let buf_str = std::str::from_utf8(&buf[..len]).unwrap();
            let msg: Message = rustc_serialize::json::decode(&buf_str).unwrap();
            match msg.payload {
                Payload::Request(_) => { self.handle_request(&mut socket, &msg) }
                Payload::Reply(_) => { self.handle_reply(&mut socket, &msg) }
            }
        }
    }


    pub fn get(&mut self, key: String) -> Result<String, &'static str> {
        Err("not implemented")
    }

    pub fn put(&mut self, key: String, val: String) -> Result<(), &'static str> {
        Err("not implemented")
    }

    fn handle_request(&mut self, socket: &mut UdpSocket, msg: &Message) {
        match msg.payload {
            Payload::Request(Request::PingRequest) => {
                let reply = Message { src: self.routes.node.clone(), token: Key::random(), payload: Payload::Reply(Reply::PingReply) };
                let encoded_reply = rustc_serialize::json::encode(&reply).unwrap();
                println!("{}", encoded_reply);
                //let sent_len = socket.send_to(&encoded_reply.into_bytes(), &msg.src.addr[..]).unwrap();
            }
            _ => { }
        }
    }

    fn handle_reply(&mut self, socket: &mut UdpSocket, msg: &Message) {
    }

    fn ping(&mut self, node: NodeInfo) -> Result<(), &'static str> {
        Err("not implemented")
    }

    fn store(&mut self, key: Key, val: String) -> Result<(), &'static str> {
        Err("not implemented")
    }

    fn find_nodes(&mut self, key: Key) -> Result<Vec<NodeInfo>, &'static str> {
        Err("not implemented")
    }

    fn find_val(&mut self, key: Key) -> Result<String, &'static str> {
        Err("not implemented")
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
    fn lookup_nodes(&self, item: Key, count: usize) -> Vec<(NodeInfo, Distance)> {
        if count == 0 {
            return Vec::new();
        }
        let mut ret = Vec::with_capacity(count);
        for bucket in &self.buckets {
            for node in bucket {
                ret.push( (node.clone(), Distance::dist(node.id, item)) );
            }
        }
        ret.sort_by(|&(_,a), &(_,b)| a.cmp(&b));
        ret.truncate(count);
        return ret;
    }

}

#[derive(Debug,Clone,RustcEncodable,RustcDecodable)]
pub struct NodeInfo {
    pub id: Key,
    pub addr: String,
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

impl Decodable for Key {
    fn decode<D: Decoder>(d: &mut D) -> Result<Key, D::Error> {
        d.read_seq(|d, len| {
            if len != KEY_LEN {
                return Err(d.error("Wrong length key!"));
            }
            let mut ret = [0; KEY_LEN];
            for i in 0..KEY_LEN {
                ret[i] = try!(d.read_seq_elt(i, Decodable::decode));
            }
            Ok(Key(ret))
        })
    }
}

impl Encodable for Key {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_seq(KEY_LEN, |s| {
            for i in 0..KEY_LEN {
                try!(s.emit_seq_elt(i, |s| self.0[i].encode(s)));
            }
            Ok(())
        })
    }
}

#[derive(Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
struct Distance([u8; KEY_LEN]);

impl Distance {
    /// XORs two Keys
    fn dist(x: Key, y: Key) -> Distance{
        let mut res = [0; KEY_LEN];
        for i in 0us..KEY_LEN {
            res[i] = x.0[i] ^ y.0[i];
        }
        Distance(res)
    }

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

impl Debug for Distance {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for x in self.0.iter() {
            try!(write!(f, "{0:02x}", x));
        }
        Ok(())
    }
}

#[derive(Debug,RustcEncodable, RustcDecodable)]
pub struct Message {
    pub src: NodeInfo,
    pub token: Key,
    pub payload: Payload,
}

#[derive(Debug,RustcEncodable, RustcDecodable)]
pub enum Payload {
    Request(Request),
    Reply(Reply),
}

#[derive(Debug,RustcEncodable, RustcDecodable)]
pub enum Request {
    PingRequest,
    StoreRequest,
    FindNodeRequest,
    FindValueRequest,
}

#[derive(Debug,RustcEncodable, RustcDecodable)]
pub enum Reply {
    PingReply,
    StoreReply,
    FindNodeReply,
    FindValueReply,
}
