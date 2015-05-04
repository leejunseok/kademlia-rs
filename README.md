Kademlia in Rust
================

Paper: http://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf

This is a basic (currently WIP) implementation in Rust of the Kademlia distributed hash table, a
peer-to-peer information system. It stores both keys and values as character strings, and does not
rely on master-slave relations; all nodes in the system are peers.

This is a personal project which I started primarily to learn Rust. Along the way I got to use some
deas from my systems programming regarding shared mutable state and programming multi-threaded
applications.

Running
=======

`RUST_LOG=info cargo run`

Once started, it will expect one line of input; the info for an existing node, formatted as follows:

    <IP>:<Port> <Key>

This is optional, and if you enter a blank line, the node will start up without bootstrapping.

Once a node starts, it will log its information (IP,Port,Key) to stdout PROVIDED THAT `RUST_LOG` IS SET TO `info` in the environment.

At this point, you can enter some commands:

    put <string key> <string value> ..store value at key
    get <string key>                ..lookup value at key
    ======\/ lower level \/======
    p <ip>:<port> <key>  ..pings the node
    s <ip>:<port> <key>  ..sends store req to node
    fn <ip>:<port> <key> ..sends find_node req to node
    fv <ip>:<port> <key> ..sends find_value req to node
    ln <key>             ..performs iterative node lookup
    lv <string key>      ..performs iterative value lookup

Note that there is a distinction between keys (20-length byte strings) and string keys (arbitrary strings).

Implementation
==============

Each Kademlia node has two large components: the node itself (represented by the `Kademlia`
struct), and the remote procedure call (RPC) facilities, represented by the `Rpc` struct. They are
a bit more tightly coupled than I would like, but I've done my best to keep them separate where
possible.

The Kademlia node includes the routing table (K buckets), the store (a basic hash map), and a
reference to an `Rpc`. The `Rpc` struct allows the node to make RPCs, and also provides a source of
incoming requests to the node. The prerequisite to both of these is `Rpc::open()`, which
takes a UdpSocket and a Rust channel Sender, and starts a new thread to parse, mux, and pass along
the incoming messages.

Of course, the end user doesn't see all this; they just have to call `Kademlia::start()` with the
appropriate arguments, and they will get back a handle to the node, and this will all happen in the
background.

Feedback
========

I'm certain there are countless mistakes in this project, and I would love to hear how I could improve
them (especially with regards to error handling).
