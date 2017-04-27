//! A [Kademlia][kademlia] nde implementation.
//!
//! [kademlia]: http://www.scs.stanford.edu/%7Edm/home/papers/kpos.pdf

use bincode;
use k_bucket::{K, KBucket, KBucketEntry};
use node_id::NodeId;
use rand;
use rpc;
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
    ///
    /// FIXME(emilio): If we want to store massive blobs, we may want to support
    /// TCP instead.
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

    /// Gets the id of the node.
    pub fn id(&self) -> &NodeId {
        &self.id
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

    /// Tries to receive a message over the network.
    ///
    /// Returns a result, either success, with the socket address we received
    /// the message from, or an error.
    pub fn recv_message(&mut self) -> io::Result<(SocketAddr, rpc::RPCMessage)> {
        let mut dest = vec![0; rpc::RPC_MESSAGE_MAX_SIZE];

        let (bytes_read, source) = self.socket.recv_from(&mut dest)?;
        let message: rpc::RPCMessage =
            match bincode::deserialize(&dest[..bytes_read]) {
                Ok(m) => m,
                Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
            };

        debug!("Got message {:?}", message);
        self.note_node(&message.sender, &source);
        Ok((source, message))
    }

    /// Gets the `k` nodes we know closer to `node_id`. This is the main search
    /// procedure for the `FIND_VALUE` and `FIND_NODE` messages.
    pub fn find_k_known_nodes_closer_to(&self, id: NodeId) -> Vec<KBucketEntry> {
        let distance = self.id.xor(&id);
        let mut ret = Vec::with_capacity(K);

        // First, collect from the closest bucket.
        let index = distance.bucket_index();
        self.buckets[index].collect_into(&mut ret);

        // Collect on adjacent buckets.
        //
        // TODO(emilio): This is what the algorithm is supposed to do, but seems
        // it could miss a few entries that are closer?
        //
        // *shrug*
        let mut delta = 1;
        while ret.len() < K {
            let mut found_to_one_side = false;
            if index >= delta {
                found_to_one_side = true;
                self.buckets[index - delta].collect_into(&mut ret);
            }
            if index + delta < self.buckets.len() {
                found_to_one_side = true;
                self.buckets[index + delta].collect_into(&mut ret);
            }

            if !found_to_one_side { // We did everything we could.
                break;
            }
            delta += 1;
        }

        // TODO(emilio): This can be somewhat expensive, I guess. We could be a
        // bit smarter.
        ret.sort_by_key(|e| self.id.xor(e.id()));

        ret.truncate(K);
        ret
    }

    /// Handles a given request message.
    pub fn handle_request(&mut self,
                          request: rpc::RequestKind,
                          sender: NodeId,
                          source: SocketAddr)
                          -> io::Result<()> {
        match request {
            rpc::RequestKind::Ping => {
                let msg = rpc::MessageKind::Response(rpc::ResponseKind::Pong);
                let msg = rpc::RPCMessage::new(self.id.clone(), msg);
                self.send_message(sender, source, msg)
            }
            rpc::RequestKind::FindNode(node_id) => {
                if node_id == self.id {
                    // That's quite a nonsensical request, since they needed our
                    // address and ID to find us.
                    return Ok(());
                }

                let nodes = self.find_k_known_nodes_closer_to(node_id);
                let response = rpc::ResponseKind::FindNode(nodes);
                let msg = rpc::MessageKind::Response(response);
                let msg = rpc::RPCMessage::new(self.id.clone(), msg);

                self.send_message(sender, source, msg)
            }
            rpc::RequestKind::Store(key, val) => {
                self.store.insert(key, val);
                Ok(())
            }
            rpc::RequestKind::FindValue(key) => {
                let response = match self.store.get(&key) {
                    Some(v) => {
                        rpc::FindValueResponse::Value(v.clone())
                    }
                    None => {
                        let nodes = self.find_k_known_nodes_closer_to(key);
                        rpc::FindValueResponse::CloserNodes(nodes)
                    }
                };

                let msg = rpc::ResponseKind::FindValue(response);
                let msg = rpc::MessageKind::Response(msg);
                let msg = rpc::RPCMessage::new(self.id.clone(), msg);
                self.send_message(sender, source, msg)
            }
        }
    }

    /// Send a message to a given node.
    pub fn send_message(&mut self,
                        _id: NodeId,
                        address: SocketAddr,
                        message: rpc::RPCMessage)
                        -> io::Result<()> {
        let mut dest = vec![];
        match bincode::serialize_into(&mut dest,
                                      &message,
                                      bincode::Bounded(rpc::RPC_MESSAGE_MAX_SIZE as u64)) {
            Ok(()) => {},
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
        };

        debug!("Sent message {:?}", message);

        self.socket.send_to(&dest, address).map(|_| {})
    }
}
