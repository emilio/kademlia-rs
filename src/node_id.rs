//! The node ids for the network.

use rand::Rng;

/// A node id, with 160 bits.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub fn xor(&self, other: &Self) -> Distance {
        let mut ret = self.clone();
        for (index, piece) in other.id.iter().enumerate() {
            ret.id[index] ^= *piece;
        }
        Distance(ret)
    }
}

/// The distance between two nodes.
pub struct Distance(NodeId);

impl Distance {
    /// Returns the index of the bucket of a given distance.
    ///
    /// This will be a number from 0 to 159.
    pub fn bucket_index(&self) -> usize {
        let mut idx = 159;
        for byte in &self.0.id {
            let mut mask: u8 = 1 << 7;
            loop {
                if *byte & mask != 0 {
                    return idx;
                }
                mask >>= 1;

                if mask == 0 {
                    break;
                }

                idx -= 1;
            }
        }
        debug_assert_eq!(idx, 0);
        idx
    }
}
