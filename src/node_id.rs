//! The node ids for the network.

use rand::Rng;

/// A node id, with 160 bits.
#[derive(Clone, Debug)]
pub struct NodeId {
    id: [u8; 20],
}

impl NodeId {
    /// Create a new random `NodeId`.
    pub fn random<R>(rng: &mut R) -> Self
        where R: Rng,
    {
        let mut id: NodeId = unsafe { ::std::mem::uninitialized() };
        rng.fill_bytes(&mut id.id);
        id
    }

    /// XOR this id with `other`, in order to compute the distance.
    pub fn xor(&self, other: &Self) -> Self {
        let mut ret = self.clone();
        for (index, piece) in other.id.iter().enumerate() {
            ret.id[index] ^= *piece;
        }
        ret
    }
}
