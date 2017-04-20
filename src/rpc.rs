//! The RPC protocol used by Kademlia.

use k_bucket::KBucketEntry;
use node_id::NodeId;
use storage;

/// A single RPC message.
pub struct RPCMessage {
    /// The sender of the message.
    sender: NodeId,
    /// The message that was sent.
    kind: MessageKind,
}

/// The different messages defined by the RPC protocol.
pub enum MessageKind {
    /// A request message.
    Request(RequestKind),
    /// A response message.
    Response(ResponseKind),
}

/// The different request kinds defined by the RPC protocol.
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
pub enum ResponseKind {
    /// A `PONG` message, as a response to a ping.
    Pong,
    /// A `FIND_NODE` response, with the node addresses close to the nodes.
    FindNode(Vec<KBucketEntry>),
    /// A `FIND_VALUE` reply, with either a value or a list of closer nodes.
    FindValue(FindValueResponse),
}

/// A response for a `FIND_VALUE`
pub enum FindValueResponse {
    /// A value was found for this key.
    Value(storage::Value),

    /// The value was not found on this node, but here are some nodes that are
    /// closer.
    CloserNodes(Vec<KBucketEntry>),
}
