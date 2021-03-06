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
    pub fn start(net_id: String, node_id: Key, node_addr: &str, bootstrap: Option<NodeInfo>) -> Kademlia {
        let socket = UdpSocket::bind(node_addr).unwrap(); // err: failed to bind to socket
        let node_info = NodeInfo {
            id: node_id.clone(),
            addr: socket.local_addr().unwrap().to_string(), // err: failed to retrieve local addr
            net_id: net_id,
        };
        let mut routes = RoutingTable::new(node_info.clone());
        if let Some(bootstrap) = bootstrap {
            routes.update(bootstrap);
        }
        info!("New node created at {} with ID {:?}", &node_info.addr, &node_info.id);

        let (tx, rx) = mpsc::channel();
        let rpc = Rpc::open(socket, tx, node_info.clone());

        let node = Kademlia {
            routes: Arc::new(Mutex::new(routes)),
            store: Arc::new(Mutex::new(HashMap::new())),
            node_info: node_info,
            rpc: Arc::new(rpc),
        };

        node.clone().start_req_handler(rx);

        node.lookup_nodes(node_id);

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
            info!("Channel closed, since sender is dead.");
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

    pub fn ping_raw(&self, dst: NodeInfo) -> Receiver<Option<Reply>> {
        self.rpc.send_req(Request::Ping, dst)
    }

    pub fn store_raw(&self, dst: NodeInfo, k: String, v: String) -> Receiver<Option<Reply>> {
        self.rpc.send_req(Request::Store(k, v), dst)
    }

    pub fn find_node_raw(&self, dst: NodeInfo, id: Key) -> Receiver<Option<Reply>> {
        self.rpc.send_req(Request::FindNode(id), dst)
    }

    pub fn find_value_raw(&self, dst: NodeInfo, k: String) -> Receiver<Option<Reply>> {
        self.rpc.send_req(Request::FindValue(k), dst)
    }

    pub fn ping(&self, dst: NodeInfo) -> Option<()> {
        let rep = self.ping_raw(dst.clone()).recv().unwrap(); // err: pending reply channel closed
        let mut routes = self.routes.lock().unwrap();
        if let Some(Reply::Ping) = rep {
            routes.update(dst);
            Some(())
        } else {
            routes.remove(&dst);
            None
        }
    }

    pub fn store(&self, dst: NodeInfo, k: String, v: String) -> Option<()> {
        let rep = self.store_raw(dst.clone(), k, v).recv().unwrap(); // err: pending reply channel closed
        let mut routes = self.routes.lock().unwrap();
        if let Some(Reply::Ping) = rep {
            routes.update(dst);
            Some(())
        } else {
            routes.remove(&dst);
            None
        }
    }

    pub fn find_node(&self, dst: NodeInfo, id: Key) -> Option<Vec<NodeAndDistance>> {
        let rep = self.find_node_raw(dst.clone(), id).recv().unwrap(); // err: pending reply channel closed
        let mut routes = self.routes.lock().unwrap();
        if let Some(Reply::FindNode(entries)) = rep {
            routes.update(dst);
            Some(entries)
        } else {
            routes.remove(&dst);
            None
        }
    }

    pub fn find_value(&self, dst: NodeInfo, k: String) -> Option<FindValueResult> {
        let rep = self.find_value_raw(dst.clone(), k).recv().unwrap(); // err: pending reply channel closed
        let mut routes = self.routes.lock().unwrap();
        if let Some(Reply::FindValue(res)) = rep {
            routes.update(dst);
            Some(res)
        } else {
            routes.remove(&dst);
            None
        }
    }

    pub fn lookup_nodes(&self, id: Key) -> Vec<NodeAndDistance> {
        let mut queried = HashSet::new();
        let mut ret = HashSet::new();

        // Add the closest nodes we know to our queue of nodes to query
        let routes = self.routes.lock().unwrap();
        let mut to_query = BinaryHeap::from(routes.closest_nodes(id, K_PARAM));
        drop(routes);

        for entry in &to_query {
            queried.insert(entry.clone());
        }

        while !to_query.is_empty() {
            let mut joins = Vec::new();
            let mut queries = Vec::new();
            let mut results = Vec::new();
            for _ in 0..A_PARAM {
                match to_query.pop() {
                    Some(entry) => { queries.push(entry); }
                    None => { break; }
                }
            }
            for &NodeAndDistance(ref ni, _) in &queries {
                let ni = ni.clone();
                let node = self.clone();
                joins.push(thread::spawn(move || {
                    node.find_node(ni.clone(), id)
                }));
            }
            for j in joins {
                results.push(j.join().unwrap());
            }
            for (res, query) in results.into_iter().zip(queries) {
                if let Some(entries) = res {
                    ret.insert(query);
                    for entry in entries {
                        if queried.insert(entry.clone()) {
                            to_query.push(entry);
                        }
                    }
                }
            }
        }

        let mut ret = ret.into_iter().collect::<Vec<_>>();
        ret.sort_by(|a,b| a.1.cmp(&b.1));
        ret.truncate(K_PARAM);
        ret
    }

    pub fn lookup_value(&self, k: String) -> (Option<String>, Vec<NodeAndDistance>) {
        let id = Key::hash(k.clone());
        let mut queried = HashSet::new();
        let mut ret = HashSet::new();

        // Add the closest nodes we know to our queue of nodes to query
        let routes = self.routes.lock().unwrap();
        let mut to_query = BinaryHeap::from(routes.closest_nodes(id, K_PARAM));
        drop(routes);

        for entry in &to_query {
            queried.insert(entry.clone());
        }

        while !to_query.is_empty() {
            let mut joins = Vec::new();
            let mut queries = Vec::new();
            let mut results = Vec::new();
            for _ in 0..A_PARAM {
                match to_query.pop() {
                    Some(entry) => { queries.push(entry); }
                    None => { break; }
                }
            }
            for &NodeAndDistance(ref ni, _) in &queries {
                let k = k.clone();
                let ni = ni.clone();
                let node = self.clone();
                joins.push(thread::spawn(move || {
                    node.find_value(ni.clone(), k)
                }));
            }
            for j in joins {
                results.push(j.join().unwrap());
            }
            for (res, query) in results.into_iter().zip(queries) {
                if let Some(fvres) = res {
                    match fvres {
                        FindValueResult::Nodes(entries) => {
                            ret.insert(query);
                            for entry in entries {
                                if queried.insert(entry.clone()) {
                                    to_query.push(entry);
                                }
                            }
                        }
                        FindValueResult::Value(val) => {
                            let mut ret = ret.into_iter().collect::<Vec<_>>();
                            ret.sort_by(|a,b| a.1.cmp(&b.1));
                            ret.truncate(K_PARAM);
                            return (Some(val), ret);
                        }
                    }
                }
            }
        }

        let mut ret = ret.into_iter().collect::<Vec<_>>();
        ret.sort_by(|a,b| a.1.cmp(&b.1));
        ret.truncate(K_PARAM);

        (None, ret)
    }

    pub fn put(&self, k: String, v: String) {
        let candidates = self.lookup_nodes(Key::hash(k.clone()));
        for NodeAndDistance(node_info, _) in candidates {
            let node = self.clone();
            let k = k.clone();
            let v = v.clone();
            thread::spawn(move || {
                node.store(node_info, k, v).unwrap();
            });
        }
    }

    pub fn get(&self, k: String) -> Option<String> {
        let (v_opt, mut nodes) = self.lookup_value(k.clone());
        v_opt.map(|v| {
            if let Some(NodeAndDistance(store_target, _)) = nodes.pop() {
                self.store(store_target, k, v.clone());
            } else {
                self.store(self.node_info.clone(), k, v.clone());
            }
            v
        })
    }

    pub fn print_routes(&self) {
        let routes = self.routes.lock().unwrap();
        routes.print();
    }
}
