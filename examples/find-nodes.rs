extern crate kademlia;
#[macro_use]
extern crate log;
extern crate env_logger;

use kademlia::node::Node;
use kademlia::{rpc, storage};
use std::sync::mpsc;
use std::thread;

const DUMB_NODES: usize = 20;

fn main() {
    log::set_logger(|max_log_level| {
        use env_logger::Logger;
        let env_logger = Logger::new();
        max_log_level.set(env_logger.filter());
        Box::new(env_logger)
    }).expect("Failed to set logger.");

    let (tx, rx) = mpsc::channel();

    // Spawn twenty nodes that only act as servers, and don't do any special
    // requests.
    for i in 0..DUMB_NODES {
        let tx = tx.clone();
        thread::spawn(move || {
            let address = format!("127.0.0.1:{}", 4302 + i);
            let mut node = Node::new(&address).unwrap();
            let address = node.address().unwrap();
            tx.send((node.id().clone(), address)).unwrap();
            while let Ok((source, message)) = node.recv_message() {
                match message.kind {
                    rpc::MessageKind::Request(r) => {
                        println!("[{:?}] Got request {:?} from {:?} at {:?}",
                                 node.id(), r, message.sender, source);
                        match node.handle_request(r, message.sender, source) {
                            Ok(..) => {
                                debug!("[{:?}] Correctly handled", node.id());
                            }
                            Err(err) => {
                                error!("[{:?}] error: {:?}", node.id(), err);
                            }
                        }
                    }
                    other => panic!("Unexpected response {:?}", other),
                }
            }
        });
    }

    let mut ids = Vec::with_capacity(DUMB_NODES);
    for _ in 0..DUMB_NODES {
        ids.push(rx.recv().unwrap());
    }

    println!("Starting node ids: {:?}", ids);

    let mut node = Node::new("127.0.0.1:4300").unwrap();
    println!("Main node: {:?}", node.id());

    // Let the other nodes know us.
    for &(ref id, ref address) in &ids {
        node.note_node(id, address);
        let msg =
            rpc::RPCMessage::new(node.id().clone(),
                                 rpc::MessageKind::Request(rpc::RequestKind::Ping));
        node.send_message(id.clone(), address.clone(), msg).unwrap();
    }

    for _ in 0..DUMB_NODES {
        let (source, message) = node.recv_message().unwrap();
        match message.kind {
            rpc::MessageKind::Response(rpc::ResponseKind::Pong) => {
                println!("Got pong from {:?} at {:?}",
                         message.sender, source);
            },
            other => panic!("unexpected message {:?}", other),
        }
    }

    // This one is going to poll until it finds the key "foo".
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut node = Node::new("127.0.0.1:4301").unwrap();
        for &(ref id, ref address) in &ids {
            node.note_node(id, address);
            let msg =
                rpc::RPCMessage::new(node.id().clone(),
                                     rpc::MessageKind::Request(rpc::RequestKind::Ping));
            node.send_message(id.clone(), address.clone(), msg).unwrap();
        }

        loop {
            match node.find(storage::hash(&"foo".into())).unwrap() {
                Some(v) => {
                    tx.send(Some(v)).unwrap();
                    break;
                },
                None => {},
            }
        }

        loop {
            // FIXME(emilio): hashing _before_ looking can't avoid collisions,
            // which sucks!
            //
            // For now just ignore them.
            match node.find(storage::hash(&"fuzzz".into())).unwrap() {
                Some(..) => panic!("How!"),
                None => {
                    tx.send(None).unwrap();
                    break;
                },
            }
        }
    });


    node.try_store(storage::hash(&"foo".into()), "bar".into());
    let value = rx.recv().unwrap();
    assert_eq!(value, Some("bar".into()));
    println!("Success! The other node found the value {:?}", value);
    let value = rx.recv().unwrap();
    assert!(value.is_none());
    println!("Success! The other node didn't found the value {:?}", value);
}
