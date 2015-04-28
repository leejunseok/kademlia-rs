use std::collections::HashMap;
use std::net::UdpSocket;
use std::str;
use std::sync::{Arc,Mutex};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver,Sender};
use std::thread;
use rustc_serialize::json;

use ::{MESSAGE_LEN,TIMEOUT};
use ::kademlia::{Message,Payload};
use ::key::Key;

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub struct RpcMessage {
    token: Key,
    pub msg: Message,
}

pub struct ReqHandle {
    pub req_msg: Message,
    token: Key,
    rpc: Rpc,
}

impl ReqHandle {
    pub fn rep(self, rep_msg: Message) {
        let rmsg = RpcMessage {
            token: self.token,
            msg: rep_msg,
        };
        self.rpc.send_msg(&rmsg, &self.req_msg.src.addr);
    }
}

#[derive(Clone)]
pub struct Rpc {
    socket: Arc<UdpSocket>,
    pending: Arc<Mutex<HashMap<Key,Sender<Option<Message>>>>>,
}

impl Rpc {
    pub fn open(socket: UdpSocket, tx: Sender<ReqHandle>, net_id: &str) -> Rpc {
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
                let rmsg = json::decode::<RpcMessage>(&buf_str).unwrap();

                println!("|  IN | {:?} <== {:?} ", rmsg.msg.payload, rmsg.msg.src.id);

                if rmsg.msg.src.net_id != net_id {
                    println!("Message from different net_id received, ignoring.");
                    continue;
                }

                match rmsg.msg.payload {
                    Payload::Kill => {
                        break;
                    }
                    Payload::Request(_) => {
                        let req_handle = ReqHandle {
                            req_msg: rmsg.msg,
                            token: rmsg.token,
                            rpc: rpc.clone(),
                        };
                        if let Err(_) = tx.send(req_handle) {
                            println!("Closing channel, since receiver is dead.");
                            break;
                        }
                    }
                    Payload::Reply(_) => {
                        rpc.clone().pass_rep(rmsg);
                    }
                }
            }
        });
        ret
    }

    /// Passes a reply received through the Rpc socket to the appropriate pending Receiver
    fn pass_rep(self, rmsg: RpcMessage) {
        thread::spawn(move || {
            let mut pending = self.pending.lock().unwrap();
            let send_res = match pending.get(&rmsg.token) {
                Some(tx) => {
                    tx.send(Some(rmsg.msg.clone()))
                }
                None => {
                    println!("Unsolicited reply received, ignoring.");
                    return;
                }
            };
            if let Ok(_) = send_res {
                pending.remove(&rmsg.token);
            }
        });
    }

    /// Sends a message
    fn send_msg(&self, rmsg: &RpcMessage, addr: &str) {
        let enc_rep = json::encode(rmsg).unwrap();
        self.socket.send_to(&enc_rep.as_bytes(), addr).unwrap();
        println!("{:?}", enc_rep);
        println!("| OUT | {:?} ==> {:?} ", rmsg.msg, addr);
    }

    /// Sends a request of data from src_info to dst_info, returning a Receiver for the reply
    pub fn send_req(&self, msg: Message, addr: &str) -> Receiver<Option<Message>> {
        let (tx, rx) = mpsc::channel();
        let mut pending = self.pending.lock().unwrap();
        let mut token = Key::random();
        while pending.contains_key(&token) {
            token = Key::random();
        }
        pending.insert(token, tx.clone());
        drop(pending);

        let rmsg = RpcMessage { 
            token: token,
            msg: msg,
        };
        self.send_msg(&rmsg, addr);

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
