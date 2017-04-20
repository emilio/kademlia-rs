//! A definition of common storage-related types.

use node_id::NodeId;
use std::collections::HashMap;

/// A key in the distributed store.
pub type Key = NodeId;

/// A value in the store.
///
/// FIXME(emilio): Right now we only store strings, but this doesn't need to be
/// true forever.
///
/// Actually, perhaps the code should be more generic across keys and values...
/// Oh well.
pub type Value = String;

/// The actual store we use in each node. Right now we use a standard `HashMap`.
///
/// We could use some persistent storage or what not.
pub type Store = HashMap<Key, Value>;
