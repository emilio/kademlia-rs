//! A K-bucket.

use node_id::NodeId;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::collections::HashSet;

/// A k-bucket entry representing a single node, with information necessary to
/// contact it.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Get the id associated with this entry.
    pub fn id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get the address associated with this entry.
    pub fn address(&self) -> &SocketAddr {
        &self.ip
    }
}

/// The `k` constant as described in the paper:
///
/// > k is chosen such that any given k nodes are very unlikely to
/// > fail within an hour of each other (for example k = 20).
///
/// In our application, since we don't have that many nodes, 6 is probably fine.
pub const K: usize = 6;

/// A k-bucket, that is, a list of up-to k entries representing the most
/// recently seen nodes in the range corresponding to this bucket.
#[derive(Debug, Serialize, Deserialize)]
pub struct KBucket {
    /// An ordered list of nodes, ordered from least-recently seen to
    /// most-recently seen.
    entries: VecDeque<KBucketEntry>,
}

impl KBucket {
    /// Trivially constructs a new `KBucket`.
    pub fn new() -> Self {
        KBucket {
            entries: VecDeque::with_capacity(K + 1),
        }
    }

    /// Collects all the entries of this bucket into `result` that are not
    /// present into `seen`.
    pub fn collect_into(&self,
                        result: &mut Vec<KBucketEntry>,
                        seen: &HashSet<NodeId>) {
        for entry in &self.entries {
            if !seen.contains(entry.id()) {
                result.push(entry.clone());
            }
        }
    }

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
            Some(i) => self.entries.remove(i).unwrap(),
            None => KBucketEntry::new(id.clone(), address.clone()),
        };

        self.entries.push_back(new_entry);

        if self.entries.len() > K {
            self.entries.pop_front()
        } else {
            None
        }
    }
}
