extern crate rand;
extern crate rustc_serialize;

use rustc_serialize::{Decodable, Encodable, Decoder, Encoder};
use rustc_serialize::json;

use std::str;
use std::sync::{Arc, Mutex};
use std::fmt::{Error, Debug, Formatter};
use std::net::UdpSocket;
use std::thread;

const K: usize = 20;
const N_BUCKETS: usize = K * 8;
const BUCKET_SIZE: usize = 20;
const MESSAGE_LEN: usize = 8196;

#[derive(Clone)]
pub struct DhtHandle {
    routes: Arc<Mutex<RoutingTable>>,
    pub net_id: String,
    rpc: Arc<RpcEndpoint>,
}

impl DhtHandle {
    pub fn new(net_id: &str, node_id: Key, node_addr: &str) -> DhtHandle {
        let socket = UdpSocket::bind(node_addr).unwrap();
        let node_info = NodeInfo {
            id: node_id,
            addr: socket.local_addr().unwrap().to_string(),
        };
        let routes = RoutingTable::new(node_info);
        println!("New node created at {:?} with ID {:?}",
                 &routes.node_info.addr,
                 &routes.node_info.id);

        DhtHandle {
            routes: Arc::new(Mutex::new(routes)),
            net_id: String::from(net_id),
            rpc: Arc::new(RpcEndpoint { socket: socket }),
        }
    }

    pub fn start(&mut self, bootstrap: &str) {
        self.rpc.start_server(&*self);
    }

    fn handle_request(&mut self, msg: &Message) -> Message {
        match msg.payload {
            Payload::Request(Request::PingRequest) => {
                let routes = self.routes.lock().unwrap();
                Message {
                    src: routes.node_info.clone(),
                    token: msg.token,
                    payload: Payload::Reply(Reply::PingReply),
                }
            }
            Payload::Request(Request::StoreRequest(ref k, ref v)) => {
                panic!("Not implemented");
            }
            Payload::Request(Request::FindNodeRequest(id)) => {
                let routes = self.routes.lock().unwrap();
                Message {
                    src: routes.node_info.clone(),
                    token: msg.token,
                    payload: Payload::Reply(Reply::FindNodeReply(routes.lookup_nodes(id, 3))),
                }
            }
            Payload::Request(Request::FindValueRequest(ref k)) => {
                panic!("Not implemented");
            }
            _ => {
                panic!("Handle request was given something that's not a request");
            }
        }
    }

    fn handle_reply(&mut self, msg: &Message) {
        match msg.payload {
            Payload::Reply(Reply::PingReply) => {
                let mut routes = self.routes.lock().unwrap();
                routes.update(msg.src.clone());
                println!("Routing table updated");
            }
            _ => { }
        }
    }
}

struct RpcEndpoint {
    socket: UdpSocket,
}

impl RpcEndpoint {
    fn start_server(&self, dht: &DhtHandle) {
        let dht = dht.clone();
        let mut buf = [0u8; MESSAGE_LEN];
        loop {
            // NOTE: We currently just trust the src in the message, and ignore where
            // it actually came from
            let (len, _) = self.socket.recv_from(&mut buf).unwrap();
            let buf_str = String::from(str::from_utf8(&buf[..len]).unwrap());
            let msg: Message = json::decode(&buf_str).unwrap();

            println!("|  IN | {:?} <== {:?} ", msg.payload, msg.src.id);

            let mut dht = dht.clone();
            match msg.payload {
                Payload::Kill => {
                    break;
                }
                Payload::Request(_) => {
                    thread::spawn(move || {
                        let reply = dht.handle_request(&msg);
                        let enc_reply = json::encode(&reply).unwrap();
                        dht.rpc.socket.send_to(&enc_reply.as_bytes(), &msg.src.addr[..]).unwrap();
                        println!("| OUT | {:?} ==> {:?} ",
                                 reply.payload,
                                 reply.src.id);
                    });
                }
                Payload::Reply(_) => {
                    thread::spawn(move || {
                        dht.handle_reply(&msg)
                    });
                }
            }
        }
    }
}

impl Clone for RpcEndpoint {
    fn clone(&self) -> RpcEndpoint {
        RpcEndpoint { socket: self.socket.try_clone().unwrap() }
    }
}

struct RoutingTable {
    node_info: NodeInfo,
    buckets: Vec<Vec<NodeInfo>>
}

impl RoutingTable {
    fn new(node_info: NodeInfo) -> RoutingTable {
        let mut buckets = Vec::new();
        for _ in 0..N_BUCKETS {
            buckets.push(Vec::new());
        }
        let mut ret = RoutingTable { node_info: node_info.clone(), buckets: buckets };
        ret.update(node_info);
        ret
    }

    /// Update the appropriate bucket with the new node's info
    fn update(&mut self, node_info: NodeInfo) {
        let bucket_index = self.lookup_bucket_index(node_info.id);
        let bucket = &mut self.buckets[bucket_index];
        let node_index = bucket.iter().position(|x| x.id == node_info.id);
        match node_index {
            Some(i) => {
                let temp = bucket.remove(i);
                bucket.push(temp);
            }
            None => {
                if bucket.len() < BUCKET_SIZE {
                    bucket.push(node_info);
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
    fn lookup_nodes(&self, item: Key, count: usize) -> Vec<NodeInfo> {
        if count == 0 {
            return Vec::new();
        }
        let mut ret = Vec::with_capacity(count);
        for bucket in &self.buckets {
            for node_info in bucket {
                ret.push( (node_info.clone(), Distance::dist(node_info.id, item)) );
            }
        }
        ret.sort_by(|&(_,a), &(_,b)| a.cmp(&b));
        ret.truncate(count);
        ret.into_iter().map(|p| p.0).collect()
    }

    fn lookup_bucket_index(&self, item: Key) -> usize {
        Distance::dist(self.node_info.id, item).zeroes_in_prefix()
    }
}

#[derive(Debug,Clone,RustcEncodable,RustcDecodable)]
pub struct NodeInfo {
    pub id: Key,
    pub addr: String,
}

#[derive(Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
pub struct Key([u8; K]);

impl Key {
    /// Returns a random, K long byte string.
    pub fn random() -> Key {
        let mut res = [0; K];
        for i in 0us..K {
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
            if len != K {
                return Err(d.error("Wrong length key!"));
            }
            let mut ret = [0; K];
            for i in 0..K {
                ret[i] = try!(d.read_seq_elt(i, Decodable::decode));
            }
            Ok(Key(ret))
        })
    }
}

impl Encodable for Key {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_seq(K, |s| {
            for i in 0..K {
                try!(s.emit_seq_elt(i, |s| self.0[i].encode(s)));
            }
            Ok(())
        })
    }
}

#[derive(Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
struct Distance([u8; K]);

impl Distance {
    /// XORs two Keys
    fn dist(x: Key, y: Key) -> Distance{
        let mut res = [0; K];
        for i in 0us..K {
            res[i] = x.0[i] ^ y.0[i];
        }
        Distance(res)
    }

    fn zeroes_in_prefix(&self) -> usize {
        for i in 0..K {
            for j in 8us..0 {
                if (self.0[i] >> (7 - j)) & 0x1 != 0 {
                    return i * 8 + j;
                }
            }
        }
        K * 8 - 1
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
    Kill,
    Request(Request),
    Reply(Reply),
}

#[derive(Debug,RustcEncodable, RustcDecodable)]
pub enum Request {
    PingRequest,
    StoreRequest(String, String),
    FindNodeRequest(Key),
    FindValueRequest(String),
}

#[derive(Debug,RustcEncodable, RustcDecodable)]
pub enum Reply {
    PingReply,
    FindNodeReply(Vec<NodeInfo>),
    FindValueReply(String),
}
