//! The node ids for the network.

use rand::Rng;
use std::cmp::{Ord, PartialOrd, Ordering};

/// A node id, with 160 bits.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Eq, PartialEq)]
pub struct Distance(NodeId);

impl Ord for Distance {
    fn cmp(&self, other: &Self) -> Ordering {
        for (one, other) in self.0.id.iter().zip(other.0.id.iter()) {
            let ordering = one.cmp(other);
            if ordering != Ordering::Equal {
                return ordering;
            }
        }

        Ordering::Equal
    }
}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[test]
fn ord_and_index() {
    let zero = NodeId {
        id: [0; 20],
    };

    let mut one = NodeId {
        id: [0; 20],
    };
    one.id[19] = 1;

    let mut two = NodeId {
        id: [0; 20],
    };
    two.id[19] = 2;

    let mut three = NodeId {
        id: [0; 20],
    };
    three.id[19] = 3;

    let mut really_big = NodeId {
        id: [0; 20],
    };
    really_big.id[0] = 2;

    let distance_to_one = zero.xor(&one);
    let distance_to_two = zero.xor(&two);
    let distance_to_three = zero.xor(&three);
    let distance_to_really_big = zero.xor(&really_big);

    assert!(distance_to_two > distance_to_one);
    assert!(distance_to_really_big > distance_to_two);
    assert_ne!(distance_to_one.bucket_index(),
               distance_to_two.bucket_index());
    assert_eq!(distance_to_three.bucket_index(),
               distance_to_two.bucket_index());
}

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

                if idx != 0 {
                    idx -= 1;
                }

                if mask == 0 {
                    break;
                }
            }
        }
        debug_assert_eq!(idx, 0);
        idx
    }
}
