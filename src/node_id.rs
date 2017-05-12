//! The node ids for the network.

use rand::Rng;
use std::cmp::{Ord, PartialOrd, Ordering};
use std::fmt;

/// A node id, with 160 bits.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId {
    id: [u8; 20],
}

impl NodeId {
    /// Creates a given ID with the raw bytes specified. Useful for creating
    /// keys.
    pub fn from_bytes(id: [u8; 20]) -> Self {
        NodeId { id }
    }

    /// Gets a node id from an hexadecimal string.
    pub fn from_hex_string(string: &str) -> Option<Self> {
        if string.is_empty() {
            return None;
        }

        let mut id = NodeId { id: [0; 20] };
        let mut chars = string.chars().rev();

        for i in 0..20 {
            for j in 0..2 {
                let c = match chars.next() {
                    Some(c) => c,
                    None => return Some(id),
                };

                match c.to_digit(16) {
                    Some(c) => {
                        id.id[20 - i - 1] |= (c << (j * 4)) as u8
                    },
                    None => return None,
                }
            }
        }

        if chars.next().is_some() {
            return None;
        }

        Some(id)
    }

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

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut all_zeros = true;
        for byte in &self.id {
            if all_zeros && *byte == 0 {
                continue;
            }

            let upper_half = (byte & 0xf0) >> 4;
            if !all_zeros || (all_zeros && upper_half != 0) {
                write!(f, "{:x}", upper_half)?;
            }

            all_zeros = false;
            let lower_half = byte & 0x0f;
            write!(f, "{:x}", lower_half)?;
        }

        Ok(())
    }
}

#[test]
fn test_node_id_from_string() {
    use rand;

    let mut id = NodeId { id: [0; 20] };
    id.id[19] = 1;

    let serialized = format!("{}", id);
    assert_eq!(serialized, "1");

    let deserialized = NodeId::from_hex_string(&serialized);
    assert_eq!(deserialized, Some(id));

    let mut rng = rand::OsRng::new().unwrap();
    for _ in 0..100 {
        let id = NodeId::random(&mut rng);
        let serialized = format!("{}", id);

        let deserialized = NodeId::from_hex_string(&serialized);
        assert_eq!(deserialized, Some(id));
    }

}
