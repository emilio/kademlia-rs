//! A definition of common storage-related types.

use node_id::NodeId;
use std::collections::HashMap;
use std::hash::{self, Hasher};
use std::mem;

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


/// Map unequivocally a given `Value` to a `Key`.
#[allow(deprecated)] // SipHasher is deprecated, oh well.
pub fn hash(val: &Value) -> Key {
    let mut hasher = hash::SipHasher::new();
    hasher.write(val.as_bytes());

    // These are 64 bit, we could use up to 160, but for now we just use these
    // 64 bits.
    //
    // That means that the distribution in our hashmap isn't going to be great,
    // but oh well.
    let hash: u64 = hasher.finish();
    let mut bytes = [0; 20];

    for i in 0..mem::size_of::<u64>() {
        bytes[i] = ((hash & (0xff << i)) >> i) as u8;
    }

    Key::from_bytes(bytes)
}
