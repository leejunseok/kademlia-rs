use std::cmp::Ordering;
use ::{N_BUCKETS,K_PARAM};
use ::key::{Distance,Key};

#[derive(Hash,Eq,PartialEq,Debug,Clone,RustcEncodable,RustcDecodable)]
pub struct NodeInfo {
    pub id: Key,
    pub addr: String,
    pub net_id: String,
}

#[derive(Debug)]
pub struct RoutingTable {
    node_info: NodeInfo,
    buckets: Vec<Vec<NodeInfo>>
}

#[derive(Eq,Hash,Clone,Debug,RustcEncodable,RustcDecodable)]
pub struct NodeAndDistance(pub NodeInfo, pub Distance);

impl PartialEq for NodeAndDistance {
    fn eq(&self, other: &NodeAndDistance) -> bool {
        self.1.eq(&other.1)
    }
}

impl PartialOrd for NodeAndDistance {
    fn partial_cmp(&self, other: &NodeAndDistance) -> Option<Ordering> {
        Some(other.1.cmp(&self.1))
    }
}

impl Ord for NodeAndDistance {
    fn cmp(&self, other: &NodeAndDistance) -> Ordering {
        other.1.cmp(&self.1)
    }
}

impl RoutingTable {
    pub fn new(node_info: NodeInfo) -> RoutingTable {
        let mut buckets = Vec::new();
        for _ in 0..N_BUCKETS {
            buckets.push(Vec::new());
        }
        let mut ret = RoutingTable {
            node_info: node_info.clone(),
            buckets: buckets
        };
        ret.update(node_info.clone());
        ret
    }

    /// Update the appropriate bucket with the new node's info
    pub fn update(&mut self, node_info: NodeInfo) {
        let bucket_index = self.lookup_bucket_index(node_info.id);
        let bucket = &mut self.buckets[bucket_index];
        let node_index = bucket.iter().position(|x| x.id == node_info.id);
        match node_index {
            Some(i) => {
                let temp = bucket.remove(i);
                bucket.push(temp);
            }
            None => {
                if bucket.len() < K_PARAM {
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
    pub fn closest_nodes(&self, item: Key, count: usize) -> Vec<NodeAndDistance> {
        if count == 0 {
            return Vec::new();
        }
        let mut ret = Vec::with_capacity(count);
        for bucket in &self.buckets {
            for node_info in bucket {
                ret.push( NodeAndDistance(node_info.clone(), node_info.id.dist(item)) );
            }
        }
        ret.sort_by(|a,b| a.1.cmp(&b.1));
        ret.truncate(count);
        ret
    }

    fn lookup_bucket_index(&self, item: Key) -> usize {
        self.node_info.id.dist(item).zeroes_in_prefix()
    }

    pub fn print(&self) {
        info!("{:?}", self.buckets);
    }
}
