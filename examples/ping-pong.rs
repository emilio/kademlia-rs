extern crate kademlia;
extern crate log;
extern crate env_logger;

use kademlia::node::Node;
use kademlia::rpc;
use std::net;
use std::sync::mpsc;

fn main() {
    log::set_logger(|max_log_level| {
        use env_logger::Logger;
        let env_logger = Logger::new();
        max_log_level.set(env_logger.filter());
        Box::new(env_logger)
    }).expect("Failed to set logger.");

    let (tx, rx) = mpsc::channel();
    ::std::thread::spawn(move || {
        let mut node = Node::new("127.0.0.1:4300").unwrap();
        tx.send(node.id().clone()).unwrap();
        while let Ok(..) = node.recv_message() {
            // Do nothing.
        }
    });

    let id = rx.recv().unwrap();
    let address = net::Ipv4Addr::new(127, 0, 0, 1);
    let address = net::SocketAddr::V4(net::SocketAddrV4::new(address, 4300));


    let mut node = Node::new("127.0.0.1:4301").unwrap();
    node.note_node(&id, &address);
    let msg =
        rpc::RPCMessage::new(node.id().clone(),
                             rpc::MessageKind::Request(rpc::RequestKind::Ping));
    node.send_message(id, address, msg).unwrap();
    node.recv_message().unwrap();
}
