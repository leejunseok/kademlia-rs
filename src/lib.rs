extern crate rand;
extern crate rustc_serialize;

use rustc_serialize::{Decodable, Encodable, Decoder, Encoder};

use std::sync::{Arc, Mutex};
use std::fmt::{Error, Debug, Formatter};
use std::net::UdpSocket;
use std::thread;

const KEY_LEN: usize = 20;
const N_BUCKETS: usize = KEY_LEN * 8;
const BUCKET_SIZE: usize = 20;
const MESSAGE_LEN: usize = 8196;

pub struct DHTEndpoint {
    routes: RoutingTable,
    pub net_id: String,
}

impl DHTEndpoint {
    pub fn start(net_id: String, node_id: Key, node_addr: String, bootstrap: String) {
        let mut dht = DHTEndpoint {
            routes: RoutingTable::new( NodeInfo { id: node_id, addr: node_addr } ),
            net_id: net_id,
        };
        let mut socket = UdpSocket::bind(&dht.routes.node.addr[..]).unwrap();
        let actual_addr = socket.local_addr().unwrap().to_string();
        if actual_addr != dht.routes.node.addr {
            dht.routes.node.addr = actual_addr;
            println!("DHTEndpoint's node address was updated to {:?}", dht.routes.node.addr);
        }

        let dht_arc = Arc::new(Mutex::new(dht));
        let mut buf = [0u8; MESSAGE_LEN];
        loop {
            let (len, src) = socket.recv_from(&mut buf).unwrap();
            let buf_str = std::str::from_utf8(&buf[..len]).unwrap();
            let msg: Message = rustc_serialize::json::decode(&buf_str).unwrap();

            println!("|  IN | {:?} <== {:?} ", msg.payload, msg.src.id);

            let cloned_dht_arc = dht_arc.clone();
            let mut cloned_socket = socket.try_clone().unwrap();
            thread::spawn(move || {
                match msg.payload {
                    Payload::Request(_) => {
                        DHTEndpoint::handle_request(cloned_dht_arc, &mut cloned_socket, &msg)
                    }
                    Payload::Reply(_) => {
                        DHTEndpoint::handle_reply(cloned_dht_arc, &mut cloned_socket, &msg)
                    }
                }
            });
        }
    }


    pub fn get(&mut self, key: String) -> Result<String, &'static str> {
        Err("not implemented")
    }

    pub fn put(&mut self, key: String, val: String) -> Result<(), &'static str> {
        Err("not implemented")
    }

    fn handle_request(dht_arc: Arc<Mutex<DHTEndpoint>>, socket: &mut UdpSocket, msg: &Message) {
        match msg.payload {
            Payload::Request(Request::PingRequest) => {
                let mut dht = dht_arc.lock().unwrap();
                let reply = Message {
                    src: dht.routes.node.clone(),
                    token: msg.token,
                    payload: Payload::Reply(Reply::PingReply)
                };
                let encoded_reply = rustc_serialize::json::encode(&reply).unwrap();
                let sent_len = socket.send_to(&encoded_reply.as_bytes(), &msg.src.addr[..]).unwrap();
                println!("| OUT | {:?} ==> {:?} ", reply.payload, reply.src.id);
                drop(dht);
            }
            _ => { }
        }
    }

    fn handle_reply(dht_arc: Arc<Mutex<DHTEndpoint>>, socket: &mut UdpSocket, msg: &Message) {
        match msg.payload {
            Payload::Reply(Reply::PingReply) => {
                let mut dht = dht_arc.lock().unwrap();
                dht.routes.update(msg.src.clone());
                println!("Routing table updated");
                drop(dht);
            }
            _ => { }
        }
    }

    fn ping(&mut self, socket: &mut UdpSocket, node: NodeInfo) -> Result<(), &'static str> {
        Err("not implemented")
    }

    fn store(&mut self, socket: &mut UdpSocket, key: Key, val: String) -> Result<(), &'static str> {
        Err("not implemented")
    }

    fn find_nodes(&mut self, socket: &mut UdpSocket, key: Key) -> Result<Vec<NodeInfo>, &'static str> {
        Err("not implemented")
    }

    fn find_val(&mut self, socket: &mut UdpSocket, key: Key) -> Result<String, &'static str> {
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
        let bucket_index = self.lookup_bucket_index(node.id);
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
        ret
    }

    fn lookup_bucket_index(&self, item: Key) -> usize {
        Distance::dist(self.node.id, item).zeroes_in_prefix()
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
    FindNodeReply,
    FindValueReply,
}
