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

/// A k-bucket entry representing a single node, with information necessary to
/// contact it.
pub struct KBucketEntry {
    /// The id of this node.
    id: NodeId,
    /// The socket address (ip, port) pair.
    ip: SocketAddr,
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
