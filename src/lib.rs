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

//! A Kademlia implementation in Rust.

#![deny(warnings)]
#![deny(missing_docs)]
#![allow(dead_code)]

extern crate bincode;
#[macro_use]
extern crate log;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate serde;

pub mod k_bucket;
pub mod node;
pub mod node_id;
pub mod rpc;
pub mod storage;
