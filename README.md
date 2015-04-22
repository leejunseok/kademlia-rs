Kademlia in Rust
================

This is a basic (currently WIP) implementation in Rust of the Kademlia distributed hash table, a
peer-to-peer information system. It stores both keys and values as character strings, and does not
rely on master-slave relations; all nodes in the system are peers.

This is a personal project which I started primarily to learn Rust, but also to prove to dem h8ers
that I can finish a side project. Along the way I got to use some ideas from my systems programming
regarding sharing mutable state and programming multi-threaded applications.

Implementation
==============

Each Kademlia node has two large components: the node itself (represented by the `Kademlia`
struct), and the remote procedure call (RPC) facilities, represented by the `Rpc` struct. They are
a bit more tightly coupled than I would like, but I've done my best to keep them separate where
possible.

The Kademlia node includes the routing table (K buckets), the store (a basic hash map), and a
reference to an `Rpc`. The `Rpc` struct allows the node to make RPCs, and also provides a source of
incoming requests to the node. The prerequisite to both of these is `Rpc::open_channel()`, which
takes a UdpSocket and a Rust channel Sender, and starts a new thread to parse, mux, and pass along
the incoming messages.

Of course, the end user doesn't see all this; they just have to call `Kademlia::start()` with the
appropriate arguments, and they will get back a handle to the node, and this will all happen in the
background.
