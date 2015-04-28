use std::collections::{HashMap,HashSet,VecDeque};
use std::net::UdpSocket;
use std::sync::{Arc,Mutex};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use rustc_serialize::{Decoder,Encodable,Encoder};

use ::{ALPHA,K};
use ::key::{Distance,Key};
use ::rpc::{ReqHandle,Rpc};
use ::routing::{NodeInfo,RoutingTable};

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub enum Request {
    PingRequest,
    StoreRequest(String, String),
    FindNodeRequest(Key),
    FindValueRequest(String),
}

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub enum FindValueResult {
    Nodes(Vec<NodeInfo>),
    Value(String),
}

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub enum Reply {
    PingReply,
    FindNodeReply(Vec<NodeInfo>),
    FindValueReply(FindValueResult),
}

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub enum Payload {
    Kill,
    Request(Request),
    Reply(Reply),
}

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub struct Message {
    pub src: NodeInfo,
    pub payload: Payload,
}

#[derive(Clone)]
pub struct Kademlia {
    routes: Arc<Mutex<RoutingTable>>,
    store: Arc<Mutex<HashMap<String, String>>>,
    rpc: Arc<Rpc>,
    node_info: NodeInfo,
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
        let routes = RoutingTable::new(node_info.clone());
        println!("New node created at {:?} with ID {:?}", &node_info.addr, &node_info.id);

        let (tx, rx) = mpsc::channel();
        let rpc = Rpc::open(socket, tx, net_id);

        let node = Kademlia {
            routes: Arc::new(Mutex::new(routes)),
            store: Arc::new(Mutex::new(HashMap::new())),
            node_info: node_info,
            rpc: Arc::new(rpc),
        };

        node.clone().start_req_handler(rx);

        node
    }

    fn start_req_handler(self, rx: Receiver<ReqHandle>) {
        thread::spawn(move || {
            for req_handle in rx.iter() {
                let node = self.clone();
                thread::spawn(move || {
                    let payload = node.handle_req(req_handle.req_msg.clone());
                    let rep = Message {
                        src: node.node_info.clone(),
                        payload: payload,
                    };
                    req_handle.rep(rep);
                });
            }
            println!("Channel closed, since sender is dead.");
        });
    }

    fn handle_req(&self, msg: Message) -> Payload {
        match msg.payload {
            Payload::Request(Request::PingRequest) => {
                let mut routes = self.routes.lock().unwrap();
                routes.update(msg.src.clone());
                drop(routes);

                Payload::Reply(Reply::PingReply)
            }
            Payload::Request(Request::StoreRequest(k, v)) => {
                let mut routes = self.routes.lock().unwrap();
                routes.update(msg.src.clone());
                drop(routes);

                let mut store = self.store.lock().unwrap();
                store.insert(k, v);

                Payload::Reply(Reply::PingReply)
            }
            Payload::Request(Request::FindNodeRequest(id)) => {
                let mut routes = self.routes.lock().unwrap();
                routes.update(msg.src.clone());

                let (ret, _): (Vec<_>, Vec<_>) = routes.closest_nodes(id, K).into_iter().unzip();
                Payload::Reply(Reply::FindNodeReply(ret))
            }
            Payload::Request(Request::FindValueRequest(k)) => {
                let hash = Key::from(k.clone());

                let mut store = self.store.lock().unwrap();
                let lookup_res = store.remove(&k);
                drop(store);

                let mut routes = self.routes.lock().unwrap();
                routes.update(msg.src.clone());
                match lookup_res {
                    Some(v) => {
                        Payload::Reply(Reply::FindValueReply(FindValueResult::Value(v)))
                    }
                    None => {
                        let routes = self.routes.lock().unwrap();
                        let (ret, _): (Vec<_>, Vec<_>) = routes.closest_nodes(hash, K)
                                                               .into_iter()
                                                               .unzip();
                        Payload::Reply(Reply::FindValueReply(FindValueResult::Nodes(ret)))
                    }
                }
            }
            _ => {
                panic!("Handle request was given something that's not a request.");
            }
        }
    }

    pub fn lookup_nodes(&self, id: Key) -> Vec<(NodeInfo,Distance)> {
        // Add the closest nodes we know to our queue of nodes to query
        let routes = self.routes.lock().unwrap();
        let mut to_query = routes.closest_nodes(id, K).into_iter().collect::<VecDeque<_>>();
        let mut known = HashSet::new();
        drop(routes);

        // While we still have nodes to query...
        //     ALPHA times...
        //         as long as we have queries to make...
        //             start a thread...
        //                 that sends a find_node rpc call.
        //                 If we get a reply...
        //                     and if we get the reply we expect...
        //                         zip it with their distances to the id ...
        //                         add the node we just queried ...
        //                         and return it.
        //     Collect results of each thread.
        //         If this query didnt time out...
        //             for each entry...
        //                 add it to known. If this is a previously unknown entry...
        //                     add it to the queue of nodes to query

        while !to_query.is_empty() {
            let mut joins = Vec::new();
            for _ in 0..ALPHA {
                if let Some((ni, d)) = to_query.pop_front() {
                    let node = self.clone();
                    joins.push(thread::spawn(move || {
                        let opt_msg = node.find_node(&ni, id).recv().unwrap();
                        if let Some(msg) = opt_msg {
                            if let Payload::Reply(Reply::FindNodeReply(rep_nodes)) = msg.payload {
                                let mut rep_nodes = rep_nodes.into_iter()
                                                             .map(|n| (n.clone(), n.id.dist(id)))
                                                             .collect::<Vec<_>>();
                                rep_nodes.push((ni, d));
                                Some(rep_nodes)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }));
                }
            }
            for j in joins {
                if let Some(ret) = j.join().unwrap() {
                    for entry in ret {
                        if known.insert(entry.clone()) {
                            to_query.push_back(entry);
                        }
                    }
                }
            }
        }

        let mut ret = known.into_iter().collect::<Vec<_>>();
        ret.sort_by(|&(_, a),&(_,b)| a.cmp(&b));
        ret
    }

    pub fn ping(&self, dst_info: &NodeInfo) -> Receiver<Option<Message>> {
        let msg = Message {
            src: self.node_info.clone(),
            payload: Payload::Request(Request::PingRequest),
        };
        self.rpc.send_req(msg, &dst_info.addr)
    }

    pub fn ping_sync(&self, dst_info: &NodeInfo) -> Option<()> {
        let res = self.ping(dst_info).recv().unwrap();
        if let Some(_) = res {
            let mut routes = self.routes.lock().unwrap();
            routes.update(dst_info.clone());
            Some(())
        } else {
            None
        }
    }

    pub fn store(&self, dst_info: &NodeInfo, k: &str, v: &str) -> Receiver<Option<Message>> {
        let msg = Message {
            src: self.node_info.clone(),
            payload: Payload::Request(Request::StoreRequest(String::from(k), String::from(v))),
        };
        self.rpc.send_req(msg, &dst_info.addr)
    }

    pub fn store_sync(&self, dst_info: &NodeInfo, k: &str, v:&str) -> Option<()> {
        let res = self.store(dst_info, k, v).recv().unwrap();
        if let Some(_) = res {
            let mut routes = self.routes.lock().unwrap();
            routes.update(dst_info.clone());
            Some(())
        } else {
            None
        }
    }

    pub fn find_node(&self, dst_info: &NodeInfo, id: Key) -> Receiver<Option<Message>> {
        let msg = Message {
            src: self.node_info.clone(),
            payload: Payload::Request(Request::FindNodeRequest(id)),
        };
        self.rpc.send_req(msg, &dst_info.addr)
    }

    pub fn find_node_sync(&self, dst_info: &NodeInfo, id: Key) -> Option<Vec<NodeInfo>> {
        let res = self.find_node(dst_info, id).recv().unwrap();
        if let Some(msg) = res {
            let mut routes = self.routes.lock().unwrap();
            routes.update(dst_info.clone());
            if let Payload::Reply(Reply::FindNodeReply(nodes)) = msg.payload {
                return Some(nodes);
            }
        }
        None
    }

    pub fn find_val(&self, dst_info: &NodeInfo, k: &str) -> Receiver<Option<Message>> {
        let msg = Message {
            src: self.node_info.clone(),
            payload: Payload::Request(Request::FindValueRequest(String::from(k))),
        };
        self.rpc.send_req(msg, &dst_info.addr)
    }
}
