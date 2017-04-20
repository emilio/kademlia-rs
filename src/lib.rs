//! A Kademlia implementation in Rust.

#![deny(warnings)]
#![deny(missing_docs)]
#![allow(dead_code)]

extern crate bincode;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate serde;

pub mod k_bucket;
pub mod node;
pub mod node_id;
pub mod rpc;
pub mod storage;
