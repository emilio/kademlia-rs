//! The RPC protocol used by Kademlia.

use k_bucket::KBucketEntry;
use node_id::NodeId;
use storage;

/// 1MB should be enough for now.
pub const RPC_MESSAGE_MAX_SIZE: usize = 1 * 1024 * 1024;

/// A single RPC message.
#[derive(Debug, Serialize, Deserialize)]
pub struct RPCMessage {
    /// The sender of the message.
    sender: NodeId,
    /// The message that was sent.
    kind: MessageKind,
}

impl RPCMessage {
    /// Gets the sender ID of the message.
    pub fn sender(&self) -> &NodeId {
        &self.sender
    }
}

/// The different messages defined by the RPC protocol.
#[derive(Debug, Serialize, Deserialize)]
pub enum MessageKind {
    /// A request message.
    Request(RequestKind),
    /// A response message.
    Response(ResponseKind),
}

/// The different request kinds defined by the RPC protocol.
#[derive(Debug, Serialize, Deserialize)]
pub enum RequestKind {
    /// A `PING` message.
    Ping,
    /// A `FIND_NODE` message.
    FindNode(NodeId),
    /// A `STORE_NODE` message.
    Store(storage::Key, storage::Value),
    /// A `FIND_VALUE` message.
    FindValue(storage::Key),
}

/// The different response kinds defined by the RPC protocol.
#[derive(Debug, Serialize, Deserialize)]
pub enum ResponseKind {
    /// A `PONG` message, as a response to a ping.
    Pong,
    /// A `FIND_NODE` response, with the node addresses close to the nodes.
    FindNode(Vec<KBucketEntry>),
    /// A `FIND_VALUE` reply, with either a value or a list of closer nodes.
    FindValue(FindValueResponse),
}

/// A response for a `FIND_VALUE`
#[derive(Debug, Serialize, Deserialize)]
pub enum FindValueResponse {
    /// A value was found for this key.
    Value(storage::Value),

    /// The value was not found on this node, but here are some nodes that are
    /// closer.
    CloserNodes(Vec<KBucketEntry>),
}
