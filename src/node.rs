//! A [Kademlia][kademlia] nde implementation.
//!
//! [kademlia]: http://www.scs.stanford.edu/%7Edm/home/papers/kpos.pdf

use k_bucket::KBucket;
use node_id::NodeId;
use rand;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use storage;

/// A node in this Kademlia network.
pub struct Node {
    /// Id of this node.
    id: NodeId,

    /// Keys and values stored by this node.
    store: storage::Store,

    /// The set of buckets for each bit of the key.
    buckets: Box<[KBucket]>,

    /// The UDP socket we're connecting to.
    socket: UdpSocket,

    /// The Os RNG that we'll use for all our random stuff, like message
    /// payloads.
    rng: rand::OsRng,
}

impl Node {
    /// Creates a new node, or returns an error if the function couldn't open
    /// the OS rng, or couldn't open the appropriate port.
    pub fn new<A>(addr: A) -> Result<Self, io::Error>
        where A: ToSocketAddrs,
    {
        let socket = UdpSocket::bind(addr)?;
        let mut rng = rand::OsRng::new()?;
        let id = NodeId::random(&mut rng);
        let mut buckets = Vec::with_capacity(160);
        for _ in 0..160 {
            buckets.push(KBucket::new());
        }
        Ok(Node {
            id: id,
            store: storage::Store::new(),
            buckets: buckets.into_boxed_slice(),
            socket: socket,
            rng: rng,
        })
    }

    /// A callback that gets executed for each message received or requested.
    ///
    /// This updates the routing tables, and potentially sends new messages.
    ///
    /// TODO(emilio): implement the "ping the evicted entry, and evict the newly
    /// added entry if it's still alive". Authors of the paper claim this is
    /// useful because long-living nodes tend to fail less. It's not too
    /// relevant for our implementation though.
    ///
    pub fn on_message(&mut self,
                      id: &NodeId,
                      address: &SocketAddr) {
        self.note_node(id, address);
    }

    /// A function used to note the ID and address of a node.
    pub fn note_node(&mut self,
                     id: &NodeId,
                     address: &SocketAddr) {
        let distance = self.id.xor(id);
        let _evicted_entry =
            self.buckets[distance.bucket_index()].saw_node(id, address);
    }
}
