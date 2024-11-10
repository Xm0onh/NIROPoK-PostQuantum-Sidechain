use p2p::EventType;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};
use libp2p::{
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use futures::stream::StreamExt;

mod wallet;
mod transaction;
mod p2p;
mod block;
mod blockchain;
mod mempool;
mod account;
mod validator;
mod hashchain;
mod config;

use blockchain::Blockchain;
use hashchain::{HashChain, HashChainMessage};
use config::*;
use transaction::{Transaction, TransactionType};
use log::info;



#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting the new Peer, {}", p2p::PEER_ID.clone());
    let (epoch_sender, mut epoch_rcv) = mpsc::unbounded_channel::<bool>();
    let (mining_sender, mut mining_rcv) = mpsc::unbounded_channel::<bool>();

    let wallet = wallet::Wallet::new().unwrap();
    let blockchain = Arc::new(Mutex::new(Blockchain::new(wallet)));

    let behavior = p2p::AppBehaviour::new().await;
    let transport = libp2p::tokio_development_transport(p2p::KEYS.clone()).expect("Failed to create transport");

    let mut swarm = SwarmBuilder::with_tokio_executor(transport, behavior, p2p::PEER_ID.clone()).build();
    
    let mut stdin: tokio::io::Lines<BufReader<tokio::io::Stdin>> = BufReader::new(stdin()).lines();

    let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0"
    .parse()
    .expect("Failed to parse listen address");

    swarm.listen_on(listen_addr).expect("Failed to listen on address");
    
     // Send an init event after 1 second
     let epoch_sender_clone = epoch_sender.clone();
     spawn(async move {
         sleep(Duration::from_secs(EPOCH_DURATION)).await;
         info!("sending epoch event");
         epoch_sender_clone.send(true).expect("can send epoch event");
     });

     let mut planner = periodic::Planner::new();
     planner.start();
    
    let mining_sender_clone = mining_sender.clone();
    planner.add(
        move || mining_sender_clone.send(true).expect("can send mining event"),
        periodic::Every::new(Duration::from_secs(1)),
    );
   
     loop {
      let evt =  {
       select! {
        line = stdin.next_line() => Some(p2p::EventType::Command(line.expect("can get line").expect("can read line from stdin"))),
        _epoch = epoch_rcv.recv() => Some(p2p::EventType::Epoch),
        _mining = mining_rcv.recv() => Some(p2p::EventType::Mining),
        event = swarm.select_next_some() => {
            match event {
                SwarmEvent::Behaviour(e) => {
                    let mut behaviour = swarm.behaviour_mut();
                    behaviour.handle_event(e, Arc::clone(&blockchain));
                    None
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {:?}", address);
                    None
                }
                _ => None
            }
        },
       }
    };

    if let Some(event) = evt {
        match event {
            EventType::Command(cmd) => {
                // TODO: handle commands
                info!("command: {:?}", cmd);
            }

            EventType::Epoch => {
                info!("epoch event");
                println!("epoch event");
                let hash_chain = HashChain::new();
                let hash_chain_message = HashChainMessage {
                    hash_chain_index: hash_chain.get_hash(EPOCH_DURATION as usize - 1),
                };
                let json = serde_json::to_string(&hash_chain_message).unwrap();
                swarm.behaviour_mut().floodsub.publish(p2p::HASH_CHAIN_TOPIC.clone(), json.as_bytes());
            }
            EventType::Mining => {
                // info!("mining event");
                // let json = serde_json::to_string("Hi").unwrap();
                // swarm.behaviour_mut().floodsub.publish(p2p::BLOCK_TOPIC.clone(), json.as_bytes());
            }
            EventType::HashChain => {
                info!("hash chain event");
                let hash_chain = HashChain::new();
                let json = serde_json::to_string(&hash_chain).unwrap();
                swarm.behaviour_mut().floodsub.publish(p2p::HASH_CHAIN_TOPIC.clone(), json.as_bytes());
            }   
        }
    }


     }

}
