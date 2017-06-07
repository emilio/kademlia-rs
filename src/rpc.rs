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

//! The RPC protocol used by Kademlia.

use k_bucket::KBucketEntry;
use node_id::NodeId;
use storage;

/// 100MB should be enough for now.
pub const RPC_MESSAGE_MAX_SIZE: usize = 100 * 1024 * 1024;

/// A single RPC message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPCMessage {
    /// The sender of the message.
    pub sender: NodeId,
    /// The message that was sent.
    pub kind: MessageKind,
}

impl RPCMessage {
    /// Trivially constructs a `RPCMessage`.
    pub fn new(sender: NodeId, kind: MessageKind) -> Self {
        RPCMessage { sender, kind }
    }
}

/// The different messages defined by the RPC protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageKind {
    /// A request message.
    Request(RequestKind),
    /// A response message.
    Response(ResponseKind),
}

/// The different request kinds defined by the RPC protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseKind {
    /// A `PONG` message, as a response to a ping.
    Pong,
    /// A `FIND_NODE` response, with the node addresses close to the nodes.
    FindNode(Vec<KBucketEntry>),
    /// A `FIND_VALUE` reply, with either a value or a list of closer nodes.
    FindValue(FindValueResponse),
}

/// A response for a `FIND_VALUE`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FindValueResponse {
    /// A value was found for this key.
    Value(storage::Key, storage::Value),

    /// The value was not found on this node, but here are some nodes that are
    /// closer.
    CloserNodes(Vec<KBucketEntry>),
}
