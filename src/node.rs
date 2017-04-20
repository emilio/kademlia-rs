//! A [Kademlia][kademlia] nde implementation.
//!
//! [kademlia]: http://www.scs.stanford.edu/%7Edm/home/papers/kpos.pdf

use node_id::NodeId;
use std::collections::HashMap;
use std::net::SocketAddr;

/// A key in the distributed store.
pub type Key = NodeId;

/// The actual store we use in each node. Right now we only store `String`s, but
/// we could store random blobs.
pub type Store = HashMap<Key, String>;

/// A node in this Kademlia network.
pub struct Node {
    /// Id of this node.
    id: NodeId,

    /// Keys and values stored by this node.
    store: Store,

    /// The set of buckets for each bit of the key.
    buckets: Box<[KBucket; 160]>,
}

impl Node {
    /// A callback that gets executed for each message received or requested.
    ///
    /// This updates the routing tables, and potentially sends new messages.
    ///
    /// TODO(emilio): implement the "ping the evicted entry, and evict the newly
    /// added entry if it's still alive". Authors of the paper claim this is
    /// useful because long-living nodes tend to fail less. It's not too
    /// relevant for our implementation though.
    pub fn on_message(&mut self,
                      partner: &NodeId,
                      address: &SocketAddr) {
        let distance = self.id.xor(partner);
        let _evicted_entry =
            self.buckets[distance.bucket_index()].saw_node(partner, address);
    }
}

/// A k-bucket entry representing a single node, with information necessary to
/// contact it.
pub struct KBucketEntry {
    /// The id of this node.
    node_id: NodeId,
    /// The socket address (ip, port) pair.
    ip: SocketAddr,
}

impl KBucketEntry {
    /// Trivially constructs a new KBucketEntry for a given node.
    pub fn new(node_id: NodeId, ip: SocketAddr) -> Self {
        KBucketEntry { node_id, ip }
    }
}

/// The `k` constant as described in the paper:
///
/// > k is chosen such that any given k nodes are very unlikely to
/// > fail within an hour of each other (for example k = 20).
///
/// In our application, since we don't have that many nodes, 6 is probably fine.
const K: usize = 6;

/// A k-bucket, that is, a list of up-to k entries representing the most
/// recently seen nodes in the range corresponding to this bucket.
pub struct KBucket {
    /// An ordered list of nodes, ordered from least-recently seen to
    /// most-recently seen.
    entries: Vec<KBucketEntry>,
}

impl KBucket {
    /// Called when the owner node saw a bucket in this node.
    ///
    /// This updates the bucket entry if it exists, and moves it to the last
    /// position, or adds a new entry.
    ///
    /// If the entry count runs bigger than `K`, returns the evicted entry from
    /// the list.
    pub fn saw_node(&mut self,
                    id: &NodeId,
                    address: &SocketAddr)
                    -> Option<KBucketEntry> {
        let existing_index =
            self.entries.iter().position(|e| e.node_id == *id);

        let new_entry = match existing_index {
            Some(i) => self.entries.remove(i),
            None => KBucketEntry::new(id.clone(), address.clone()),
        };

        self.entries.push(new_entry);

        if self.entries.len() > K {
            self.entries.pop()
        } else {
            None
        }
    }
}
