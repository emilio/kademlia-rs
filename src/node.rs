//! A [Kademlia][kademlia] nde implementation.
//!
//! [kademlia]: http://www.scs.stanford.edu/%7Edm/home/papers/kpos.pdf

use bincode;
use k_bucket::KBucket;
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
    pub fn recv_message(&mut self) -> io::Result<()> {
        let mut dest = vec![0; rpc::RPC_MESSAGE_MAX_SIZE];

        let (bytes_read, source) = self.socket.recv_from(&mut dest)?;
        let message = match bincode::deserialize(&dest[..bytes_read]) {
            Ok(m) => m,
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
        };

        debug!("Got message {:?}", message);

        self.handle_message(message, source)
    }

    /// Handles a given message in an appropriate way.
    pub fn handle_message(&mut self,
                          message: rpc::RPCMessage,
                          source: SocketAddr)
                          -> io::Result<()> {
        self.note_node(&message.sender, &source);

        match message.kind {
            rpc::MessageKind::Request(request) => {
                self.handle_request(request, message.sender, source)
            }
            rpc::MessageKind::Response(response) => {
                self.handle_response(response, message.sender, source)
            }
        }
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
            _ => Ok(()), // TODO
        }
    }

    /// Handles a given response message.
    pub fn handle_response(&mut self,
                           _response: rpc::ResponseKind,
                           _sender: NodeId,
                           _source: SocketAddr)
                           -> io::Result<()> {
        Ok(())
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
