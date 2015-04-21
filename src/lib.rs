extern crate rand;
extern crate rustc_serialize;

use rustc_serialize::{Decodable, Encodable, Decoder, Encoder};
use rustc_serialize::json;

use std::str;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::fmt::{Error, Debug, Formatter};
use std::net::UdpSocket;
use std::thread;

const K: usize = 20;
const N_BUCKETS: usize = K * 8;
const BUCKET_SIZE: usize = 20;
const MESSAGE_LEN: usize = 8196;
const TIMEOUT: u32 = 5000;

#[derive(Clone)]
pub struct Kademlia {
    routes: Arc<Mutex<RoutingTable>>,
    store: Arc<Mutex<HashMap<String, String>>>,
    rpc: Arc<Rpc>,
    pub node_info: NodeInfo,
}

/// A Kademlia node
impl Kademlia {
    pub fn start(net_id: &str, node_id: Key, node_addr: &str, bootstrap: &str) -> Kademlia {
        let socket = UdpSocket::bind(node_addr).unwrap();
        let node_info = NodeInfo {
            id: node_id,
            addr: socket.local_addr().unwrap().to_string(),
            net_id: String::from(net_id),
        };
        let routes = RoutingTable::new(&node_info);
        println!("New node created at {:?} with ID {:?}", &node_info.addr, &node_info.id);

        let (tx, rx) = mpsc::channel();
        let rpc = Rpc::open_channel(socket, tx, net_id);

        let node = Kademlia {
            routes: Arc::new(Mutex::new(routes)),
            store: Arc::new(Mutex::new(HashMap::new())),
            node_info: node_info.clone(),
            rpc: Arc::new(rpc),
        };

        node.start_req_handler(rx);

        let dst = NodeInfo {
            id: Key([0; K]),
            addr: String::from("127.0.0.1:50001"),
            net_id: String::from(net_id),
        };
        let ping_result = node.ping(&dst);
        println!("{:?}", ping_result.recv());

        node
    }

    fn start_req_handler(&self, rx: Receiver<Message>) {
        let node = self.clone();
        thread::spawn(move || {
            for msg in rx.iter() {
                let node = node.clone();
                let msg = msg.clone();
                thread::spawn(move || {
                    let payload = node.handle_request(&msg);
                    node.rpc.send_reply(payload, &node.node_info, &msg.src, &msg);
                });
            }
        });
    }

    fn handle_request(&self, msg: &Message) -> Payload {
        match msg.payload {
            Payload::Request(Request::PingRequest) => {
                Payload::Reply(Reply::PingReply)
            }
            Payload::Request(Request::StoreRequest(ref k, ref v)) => {
                let mut store = self.store.lock().unwrap();
                store.insert(k.clone(), v.clone());
                Payload::Reply(Reply::PingReply)
            }
            Payload::Request(Request::FindNodeRequest(id)) => {
                let routes = self.routes.lock().unwrap();
                Payload::Reply(Reply::FindNodeReply(routes.lookup_nodes(id, K)))
            }
            Payload::Request(Request::FindValueRequest(ref k)) => {
                panic!("Not implemented");
            }
            _ => {
                panic!("Handle request was given something that's not a request");
            }
        }
    }

    pub fn ping(&self, dst_info: &NodeInfo) -> Receiver<Option<Message>> {
        self.rpc.send_request(Payload::Request(Request::PingRequest),
                         &self.node_info,
                         &dst_info)
    }

    pub fn store(&self, dst_info: &NodeInfo, k: &str, v: &str) -> Receiver<Option<Message>> {
        self.rpc.send_request(Payload::Request(Request::StoreRequest(String::from(k), String::from(v))),
                         &self.node_info,
                         &dst_info)
    }

    pub fn find_node(&self, dst_info: &NodeInfo, id: Key) -> Receiver<Option<Message>> {
        self.rpc.send_request(Payload::Request(Request::FindNodeRequest(id)),
                         &self.node_info,
                         &dst_info)
    }

    pub fn find_val(&self, dst_info: &NodeInfo, k: &str) -> Receiver<Option<Message>> {
        self.rpc.send_request(Payload::Request(Request::FindValueRequest(String::from(k))),
                         &self.node_info,
                         &dst_info)
    }
}

#[derive(Clone)]
struct Rpc {
    socket: Arc<UdpSocket>,
    pending: Arc<Mutex<HashMap<Key,Sender<Option<Message>>>>>,
}

impl Rpc {
    fn open_channel(socket: UdpSocket, tx: Sender<Message>, net_id: &str) -> Rpc {
        let rpc = Rpc {
            socket: Arc::new(socket),
            pending: Arc::new(Mutex::new(HashMap::new())),
        };
        let ret = rpc.clone();
        let net_id = String::from(net_id);
        thread::spawn(move || {
            let mut buf = [0u8; MESSAGE_LEN];
            loop {
                // NOTE: We currently just trust the src in the message, and ignore where
                // it actually came from
                let (len, _) = rpc.socket.recv_from(&mut buf).unwrap();
                let buf_str = String::from(str::from_utf8(&buf[..len]).unwrap());
                let msg: Message = json::decode(&buf_str).unwrap();

                println!("|  IN | {:?} <== {:?} ", msg.payload, msg.src.id);

                if msg.src.net_id != net_id {
                    println!("Message from different net_id received, ignoring");
                    continue;
                }

                match msg.payload {
                    Payload::Kill => {
                        break;
                    }
                    Payload::Request(_) => {
                        if let Err(_) = tx.send(msg) {
                            println!("Kademlia node request handler stopped, RPC channel closing...");
                            break;
                        }
                    }
                    Payload::Reply(_) => {
                        let rpc = rpc.clone();
                        thread::spawn(move || {
                            let mut pending = rpc.pending.lock().unwrap();
                            let send_res = match pending.get(&msg.token) {
                                Some(tx) => {
                                    tx.send(Some(msg.clone()))
                                }
                                None => {
                                    println!("Unsolicited reply received, ignoring.");
                                    return;
                                }
                            };
                            if let Ok(_) = send_res {
                                pending.remove(&msg.token);
                            }
                        });
                    }
                }
            }
        });
        ret
    }

    fn send_message(&self, msg: &Message, addr: &str) {
        let enc_reply = json::encode(msg).unwrap();
        self.socket.send_to(&enc_reply.as_bytes(), addr).unwrap();
        println!("{:?}", enc_reply);
        println!("| OUT | {:?} ==> {:?} ", msg.payload, addr);
    }

    fn send_reply(&self, data: Payload, src_info: &NodeInfo, dst_info: &NodeInfo, orig: &Message) {
        let msg = Message {
            src: src_info.clone(),
            token: orig.token,
            payload: data,
        };
        self.send_message(&msg, &dst_info.addr);
    }

    fn send_request(&self, data: Payload, src_info: &NodeInfo, dst_info: &NodeInfo) -> Receiver<Option<Message>> {
        let (tx, rx) = mpsc::channel();
        let mut pending = self.pending.lock().unwrap();
        let mut token = Key::random();
        while pending.contains_key(&token) {
            token = Key::random();
        }
        pending.insert(token, tx.clone());
        drop(pending);

        let msg = Message { 
            src: src_info.clone(),
            token: token,
            payload: data,
        };
        self.send_message(&msg, &dst_info.addr);

        let rpc = self.clone();
        thread::spawn(move || {
            thread::sleep_ms(TIMEOUT);
            if let Ok(_) = tx.send(None) {
                let mut pending = rpc.pending.lock().unwrap();
                pending.remove(&token);
            }
            println!("timeout :(");
        });
        rx
    }
}

struct RoutingTable {
    node_info: NodeInfo,
    buckets: Vec<Vec<NodeInfo>>
}

impl RoutingTable {
    fn new(node_info: &NodeInfo) -> RoutingTable {
        let mut buckets = Vec::new();
        for _ in 0..N_BUCKETS {
            buckets.push(Vec::new());
        }
        let mut ret = RoutingTable {
            node_info: node_info.clone(),
            buckets: buckets
        };
        ret.update(&node_info);
        ret
    }

    /// Update the appropriate bucket with the new node's info
    fn update(&mut self, node_info: &NodeInfo) {
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
                    bucket.push(node_info.clone());
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
    pub net_id: String,
}

#[derive(Hash,Ord,PartialOrd,Eq,PartialEq,Copy,Clone)]
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

#[derive(Clone,Debug,RustcEncodable, RustcDecodable)]
pub struct Message {
    pub src: NodeInfo,
    pub token: Key,
    pub payload: Payload,
}

#[derive(Clone,Debug,RustcEncodable, RustcDecodable)]
pub enum Payload {
    Kill,
    Request(Request),
    Reply(Reply),
}

#[derive(Clone,Debug,RustcEncodable, RustcDecodable)]
pub enum Request {
    PingRequest,
    StoreRequest(String, String),
    FindNodeRequest(Key),
    FindValueRequest(String),
}

#[derive(Clone,Debug,RustcEncodable, RustcDecodable)]
pub enum Reply {
    PingReply,
    FindNodeReply(Vec<NodeInfo>),
    FindValueReply(String),
}
