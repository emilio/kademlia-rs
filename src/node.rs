/*
 * Kademlia.rs - A WIP Kademlia algorithm implementation in Rust.
 *
 * Copyright (C) 2017 Emilio Cobos Álvarez <emilio@crisal.io>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! A [Kademlia][kademlia] nde implementation.
//!
//! [kademlia]: http://www.scs.stanford.edu/%7Edm/home/papers/kpos.pdf

use bincode;
use k_bucket::{K, KBucket, KBucketEntry};
use node_id::NodeId;
use rand;
use rpc;
use std::io;
use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use storage;

/// An interface in order to handle a given message.
pub trait MessageHandler : Send {
    /// Handle a given message, possibly taking ownership of it.
    ///
    /// The `message` variable is guaranteed to be non-`None`.
    ///
    /// If it's taken, other handlers won't see the message.
    fn handle_message(&mut self, from: &SocketAddr, message: &mut Option<rpc::RPCMessage>);
}

/// A token identifying a message handler, which must be kept in order for the
/// handler to be removed.
pub struct HandlerToken(usize);

/// A node in this Kademlia network.
pub struct Node {
    /// Id of this node.
    id: NodeId,

    /// Keys and values stored by this node.
    store: storage::Store,

    /// The set of buckets for each bit of the key.
    buckets: Box<[KBucket]>,

    /// The message handlers this node owns.
    handlers: Vec<Box<MessageHandler>>,

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
            handlers: vec![],
            socket: socket,
            rng: rng,
        })
    }

    /// Gets the id of the node.
    pub fn id(&self) -> &NodeId {
        &self.id
    }

    /// Gets a view on the buckets of the node, mostly for debugging.
    pub fn buckets(&self) -> &[KBucket] {
        &self.buckets
    }

    /// Go through the raw storage mechanism.
    pub fn store(&self) -> &storage::Store {
        &self.store
    }

    /// Get the socket address of the node, if any, or an error.
    pub fn address(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// A callback that gets executed for each message received or requested.
    ///
    /// This updates the routing tables, and potentially sends new messages.
    ///
    /// TODO(emilio): implement the "ping the evicted entry, and evict the newly
    /// added entry if it's still alive". Authors of the paper claim this is
    /// useful because long-living nodes tend to fail less. It's not too
    /// relevant for our implementation though.
    pub fn on_message(&mut self,
                      id: &NodeId,
                      address: &SocketAddr) {
        self.note_node(id, address);
    }

    /// Add a handler for receiving messages sent to this node.
    // pub fn add_handler(&mut self, handler: Box<MessageHandler>) -> HandlerToken {
    //     use std::mem::transmute;
    //     let token = HandlerToken(unsafe { transmute(&*handler as *const MessageHandler) });
    //     self.handlers.push(handler);
    //     token
    // }

    /// Remove a handler from this node's address.
    // pub fn remove_handler(&mut self, token: HandlerToken) -> bool {
    //     use std::mem::transmute;

    //     let init_len = self.handlers.len();
    //     self.handlers.retain(|h| token.0 == transmute(&**h as *const MessageHandler));
    //     init_len != self.handlers.len()
    // }

    /// A function used to note the ID and address of a node.
    pub fn note_node(&mut self,
                     id: &NodeId,
                     address: &SocketAddr) {
        trace!("[{}] note_node: {} at {:?}", self.id, id, address);
        let distance = self.id.xor(id);
        let _evicted_entry =
            self.buckets[distance.bucket_index()].saw_node(id, address);
    }

    /// Set the read timeout of the underlying socket.
    pub fn set_read_timeout(&mut self,
                            duration: Option<Duration>)
                            -> io::Result<()> {
        self.socket.set_read_timeout(duration)
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


    /// Loop infinitely, running handlers as needed.
    ///
    /// TODO(emilio): This is not finished yet. This would be a slightly nicer
    /// interface, but I haven't time for it r/n.
    pub fn run_main_loop<F, U>(&mut self, mut on_error: F, mut after_message: U)
        where F: FnMut(io::Error) -> bool,
              U: FnMut(bool) -> bool,
    {

        loop {
            let (from, msg) = match self.recv_message() {
                Ok(msg) => msg,
                Err(e) => {
                    if on_error(e) {
                        break;
                    }
                    continue;
                }
            };

            let mut msg = Some(msg);
            for handler in self.handlers.iter_mut() {
                handler.handle_message(&from, &mut msg);
                if msg.is_none() {
                    continue;
                }
            }

            after_message(msg.is_some());

            if let Some(rpc::RPCMessage { kind, sender }) = msg {
                match kind {
                    rpc::MessageKind::Request(request_kind) => {
                        let _ =
                            self.handle_request(request_kind, sender, from);
                    }
                    other => {
                        debug!("Eating message: {:?}", other);
                    }
                }
            }
        }
    }

    /// Gets the `k` nodes we know closer to `node_id`. This is the main search
    /// procedure for the `FIND_VALUE` and `FIND_NODE` messages.
    pub fn find_k_known_nodes_closer_to(&self, id: &NodeId) -> Vec<KBucketEntry> {
        self.find_k_known_nodes_closer_to_not_in(id, &HashSet::new())
    }

    /// Gets the `k` nodes we know closer to `node_id`. This is the main search
    /// procedure for the `FIND_VALUE` and `FIND_NODE` messages.
    pub fn find_k_known_nodes_closer_to_not_in(&self,
                                               id: &NodeId,
                                               seen: &HashSet<NodeId>)
                                               -> Vec<KBucketEntry> {
        let distance = self.id.xor(id);
        let mut ret = Vec::with_capacity(K);

        // First, collect from the closest bucket.
        let index = distance.bucket_index();
        self.buckets[index].collect_into(&mut ret, seen);

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
                self.buckets[index - delta].collect_into(&mut ret, seen);
            }
            if index + delta < self.buckets.len() {
                found_to_one_side = true;
                self.buckets[index + delta].collect_into(&mut ret, seen);
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

                let nodes = self.find_k_known_nodes_closer_to(&node_id);
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
                        rpc::FindValueResponse::Value(key, v.clone())
                    }
                    None => {
                        let nodes = self.find_k_known_nodes_closer_to(&key);
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
            Err(err) => {
                debug!("Error sending message: {:?}", err);
                return Err(io::Error::new(io::ErrorKind::Other, err))
            }
        };

        debug!("Sent message {:?}", message);

        self.socket.send_to(&dest, address).map(|_| {})
    }

    /// Sends a store message, using the given key and value.
    ///
    /// Returns the results of the IO operations.
    pub fn try_store(&mut self,
                     key: storage::Key,
                     value: storage::Value) {
        self.store.insert(key.clone(), value.clone());

        let nodes = self.find_k_known_nodes_closer_to(&key);
        if nodes.is_empty() {
            return;
        }

        let message = rpc::MessageKind::Request(rpc::RequestKind::Store(key, value));
        let message = rpc::RPCMessage::new(self.id().clone(), message);

        for node in nodes {
            match self.send_message(node.id().clone(),
                                    node.address().clone(),
                                    message.clone()) {
                Ok(()) => {}
                Err(err) => {
                    error!("Failed to send store request to {:?}, {:?}",
                           node.id(), err);
                }
            }
        }
    }

    /// Tries to find a key in the map.
    ///
    /// Returns an error in the case of an error receiving a message, otherwise
    /// returns the value if found.
    pub fn find(&mut self,
                k: storage::Key)
                -> io::Result<Option<storage::Value>> {
        trace!("[{}] Looking at {:?}", self.id(), k);

        if let Some(r) = self.store.get(&k) {
            return Ok(Some(r.clone()));
        }

        let mut nodes_seen = HashSet::new();

        let old_timeout = self.socket.read_timeout()?;
        self.socket.set_read_timeout(None)?;

        let request =
            rpc::MessageKind::Request(rpc::RequestKind::FindValue(k.clone()));
        let request =
            rpc::RPCMessage::new(self.id.clone(), request);
        let mut nodes_to_try_from_last_round = Vec::new();
        loop {
            let nodes =
                self.find_k_known_nodes_closer_to_not_in(&k, &nodes_seen);

            trace!("[{}] closer_nodes: {:?}, from_last: {:?}", self.id(),
                   nodes, nodes_to_try_from_last_round);

            if nodes.is_empty() && nodes_to_try_from_last_round.is_empty() {
                self.socket.set_read_timeout(old_timeout)?;
                return Ok(None);
            }
            for node in nodes.into_iter().chain(nodes_to_try_from_last_round.into_iter()) {
                nodes_seen.insert(node.id().clone());
                let _ = self.send_message(node.id().clone(),
                                          node.address().clone(),
                                          request.clone());
            }

            nodes_to_try_from_last_round = Vec::new();

            // FIXME(emilio): This blocks, which is suboptimal, and assumes that
            // a timeout hasn't been reached.
            //
            // A good step would be just recv_message with a timeout. Even
            // better would be making a generic "observer" interface that
            // observed new messages and resolved a future with the value if
            // found...
            let (source, message) = self.recv_message()?;
            match message.kind {
                rpc::MessageKind::Request(r) => {
                    let _ = self.handle_request(r, message.sender, source);
                }
                rpc::MessageKind::Response(rpc::ResponseKind::FindValue(fvr)) => {
                    match fvr {
                        rpc::FindValueResponse::Value(key, v) => {
                            trace!("Got Value({:?}, {:?})", key, v);
                            if key == k {
                                self.socket.set_read_timeout(old_timeout)?;
                                return Ok(Some(v))
                            }
                            debug!("Received stale value for key {:?}", key);
                        }
                        // FIXME(emilio): This should probably reply w/ the key
                        // too to avoid stale responses?
                        rpc::FindValueResponse::CloserNodes(nodes) => {
                            nodes_to_try_from_last_round = nodes;
                        }
                    }
                }
                rpc::MessageKind::Response(..) => {
                    // FIXME(emilio): With the non-blocking, observer-based
                    // interface, this would probably be way nicer.
                    debug!("Received stale response {:?}", message);
                }
            }
        }
    }
}
