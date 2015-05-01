use std::collections::{HashMap,HashSet,BinaryHeap};
use std::net::UdpSocket;
use std::sync::{Arc,Mutex};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use rustc_serialize::{Decoder,Encodable,Encoder};

use ::{A_PARAM,K_PARAM};
use ::key::Key;
use ::rpc::{ReqHandle,Rpc};
use ::routing::{NodeAndDistance,NodeInfo,RoutingTable};

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub enum Request {
    Ping,
    Store(String, String),
    FindNode(Key),
    FindValue(String),
}

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub enum FindValueResult {
    Nodes(Vec<NodeAndDistance>),
    Value(String),
}

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub enum Reply {
    Ping,
    FindNode(Vec<NodeAndDistance>),
    FindValue(FindValueResult),
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
        println!("New node created at {} with ID {:?}", &node_info.addr, &node_info.id);

        let (tx, rx) = mpsc::channel();
        let rpc = Rpc::open(socket, tx, node_info.clone());

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
                    let rep = node.handle_req(req_handle.get_req().clone(),
                                              req_handle.get_src().clone());
                    req_handle.rep(rep);
                });
            }
            println!("Channel closed, since sender is dead.");
        });
    }

    fn handle_req(&self, req: Request, src: NodeInfo) -> Reply {
        let mut routes = self.routes.lock().unwrap();
        routes.update(src);
        drop(routes);
        match req {
            Request::Ping => {
                Reply::Ping
            }
            Request::Store(k, v) => {
                let mut store = self.store.lock().unwrap();
                store.insert(k, v);

                Reply::Ping
            }
            Request::FindNode(id) => {
                let routes = self.routes.lock().unwrap();
                Reply::FindNode(routes.closest_nodes(id, K_PARAM))
            }
            Request::FindValue(k) => {
                let hash = Key::hash(k.clone());

                let mut store = self.store.lock().unwrap();
                let lookup_res = store.remove(&k);
                drop(store);

                match lookup_res {
                    Some(v) => {
                        Reply::FindValue(FindValueResult::Value(v))
                    }
                    None => {
                        let routes = self.routes.lock().unwrap();
                        Reply::FindValue(FindValueResult::Nodes(routes.closest_nodes(hash, K_PARAM)))
                    }
                }
            }
        }
    }

    pub fn lookup_nodes(&self, id: Key) -> Vec<NodeAndDistance> {
        let mut known = HashSet::new();

        // Add the closest nodes we know to our queue of nodes to query
        let routes = self.routes.lock().unwrap();
        let mut to_query = BinaryHeap::from_vec(routes.closest_nodes(id, K_PARAM));
        drop(routes);

        while !to_query.is_empty() {
            let mut joins = Vec::new();
            let mut queries = Vec::new();
            let mut results = Vec::new();
            for _ in 0..A_PARAM {
                if let Some(entry) = to_query.pop() {
                    queries.push(entry);
                } else {
                    break;
                }
            }
            for NodeAndDistance(ni, d) in queries.into_iter() {
                let node = self.clone();
                joins.push(thread::spawn(move || {
                    let res = node.find_node(ni.clone(), id);
                    res.map(|mut nodes| {
                        nodes.push(NodeAndDistance(ni,d));
                        nodes
                    })
                }));
            }
            for j in joins {
                if let Some(res) = j.join().unwrap() {
                    results.push(res);
                }
            }
            for res in results {
                for entry in res {
                    if known.insert(entry.clone()) {
                        to_query.push(entry);
                    }
                }
            }
        }

        let mut ret = known.into_iter().collect::<Vec<_>>();
        ret.sort_by(|a,b| a.1.cmp(&b.1));
        ret
    }

    pub fn lookup_value(&self, k: String) -> FindValueResult {
        let id = Key::hash(k.clone());
        let mut known = HashSet::new();

        // Add the closest nodes we know to our queue of nodes to query
        let routes = self.routes.lock().unwrap();
        let mut to_query = routes.closest_nodes(id, K_PARAM).into_iter().collect::<BinaryHeap<_>>();
        drop(routes);

        while !to_query.is_empty() {
            let mut joins = Vec::new();
            let mut queries = Vec::new();
            let mut results = Vec::new();
            for _ in 0..A_PARAM {
                if let Some(entry) = to_query.pop() {
                    queries.push(entry);
                } else {
                    break;
                }
            }
            for NodeAndDistance(ni, d) in queries.into_iter() {
                let node = self.clone();
                let k = k.clone();
                joins.push(thread::spawn(move || {
                    let fvr_opt = node.find_value(ni.clone(), &k);
                    fvr_opt.map(|fvr| {
                        if let FindValueResult::Nodes(mut res) = fvr {
                            res.push(NodeAndDistance(ni,d));
                            FindValueResult::Nodes(res)
                        } else {
                            fvr
                        }
                    })
                }));
            }
            for j in joins {
                if let Some(res) = j.join().unwrap() {
                    results.push(res);
                }
            }
            for res in results {
                match res {
                    FindValueResult::Nodes(entries) => {
                        for entry in entries {
                            if known.insert(entry.clone()) {
                                to_query.push(entry);
                            }
                        }
                    }
                    FindValueResult::Value(val) => {
                        return FindValueResult::Value(val);
                    }
                }
            }
        }

        let mut ret = known.into_iter().collect::<Vec<_>>();
        ret.sort_by(|a,b| a.1.cmp(&b.1));
        FindValueResult::Nodes(ret)
    }

    pub fn ping_raw(&self, addr: &str) -> Receiver<Option<Reply>> {
        self.rpc.send_req(Request::Ping, addr)
    }

    pub fn store_raw(&self, addr: &str, k: &str, v: &str) -> Receiver<Option<Reply>> {
        self.rpc.send_req(Request::Store(String::from(k), String::from(v)), addr)
    }

    pub fn find_node_raw(&self, addr: &str, id: Key) -> Receiver<Option<Reply>> {
        self.rpc.send_req(Request::FindNode(id), addr)
    }

    pub fn find_value_raw(&self, addr: &str, k: &str) -> Receiver<Option<Reply>> {
        self.rpc.send_req(Request::FindValue(String::from(k)), addr)
    }

    pub fn ping(&self, dst: NodeInfo) -> Option<()> {
        let rep = self.ping_raw(&dst.addr).recv().unwrap();
        if let Some(Reply::Ping) = rep {
            let mut routes = self.routes.lock().unwrap();
            routes.update(dst);
            Some(())
        } else {
            None
        }
    }

    pub fn store(&self, dst: NodeInfo, k: &str, v:&str) -> Option<()> {
        let rep = self.store_raw(&dst.addr, k, v).recv().unwrap();
        if let Some(Reply::Ping) = rep {
            let mut routes = self.routes.lock().unwrap();
            routes.update(dst);
            Some(())
        } else {
            None
        }
    }

    pub fn find_node(&self, dst: NodeInfo, id: Key) -> Option<Vec<NodeAndDistance>> {
        let rep = self.find_node_raw(&dst.addr, id).recv().unwrap();
        if let Some(Reply::FindNode(entries)) = rep {
            let mut routes = self.routes.lock().unwrap();
            routes.update(dst);
            Some(entries)
        } else {
            None
        }
    }

    pub fn find_value(&self, dst: NodeInfo, k: &str) -> Option<FindValueResult> {
        let rep = self.find_value_raw(&dst.addr, k).recv().unwrap();
        if let Some(Reply::FindValue(res)) = rep {
            let mut routes = self.routes.lock().unwrap();
            routes.update(dst);
            Some(res)
        } else {
            None
        }
    }
}
