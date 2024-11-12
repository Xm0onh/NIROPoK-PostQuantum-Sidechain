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
mod accounts;
mod validator;
mod hashchain;
mod config;
mod utils;
mod epoch;
mod genesis;


use blockchain::Blockchain;
use hashchain::HashChain;
use config::*;
use transaction::{Transaction, TransactionType};
use log::info;
use accounts::Account;
use genesis::Genesis;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting the new Peer, {}", p2p::PEER_ID.clone());
    let (epoch_sender, mut epoch_rcv) = mpsc::unbounded_channel::<bool>();
    let (mining_sender, mut mining_rcv) = mpsc::unbounded_channel::<bool>();
    let (genesis_sender, mut genesis_rcv) = mpsc::unbounded_channel::<bool>();

    let wallet = wallet::Wallet::new().unwrap();
    let blockchain = Arc::new(Mutex::new(Blockchain::new(wallet)));
    // Lock the blockchain once


    let behavior = p2p::AppBehaviour::new().await;
    let transport = libp2p::tokio_development_transport(p2p::KEYS.clone()).expect("Failed to create transport");

    let mut swarm = SwarmBuilder::with_tokio_executor(transport, behavior, p2p::PEER_ID.clone()).build();
    
    let mut stdin: tokio::io::Lines<BufReader<tokio::io::Stdin>> = BufReader::new(stdin()).lines();

    let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0"
    .parse()
    .expect("Failed to parse listen address");

    swarm.listen_on(listen_addr).expect("Failed to listen on address");
    
    // Send a genesis event after 1 second
    let genesis_sender_clone = genesis_sender.clone();
    spawn(async move {
        sleep(Duration::from_secs(5)).await;
        info!("sending genesis event");
        genesis_sender_clone.send(true).expect("can't send genesis event");
    });


     // Send an init event after 1 second
    //  let epoch_sender_clone = epoch_sender.clone();
    //  spawn(async move {
    //      sleep(Duration::from_secs(1)).await;
    //      info!("sending epoch event");
    //      epoch_sender_clone.send(true).expect("can't send epoch event");
    //  });

     let mut planner = periodic::Planner::new();
     planner.start();
    
    // let mining_sender_clone = mining_sender.clone();
    // planner.add(
    //     move || mining_sender_clone.send(true).expect("can't send mining event"),
    //     periodic::Every::new(Duration::from_secs(1)),
    // );
   
     loop {
      let evt =  {
       select! {
        line = stdin.next_line() => Some(p2p::EventType::Command(line.expect("can get line").expect("can read line from stdin"))),
        _epoch = epoch_rcv.recv() => Some(p2p::EventType::Epoch),
        _mining = mining_rcv.recv() => Some(p2p::EventType::Mining),
        _genesis = genesis_rcv.recv() => Some(p2p::EventType::Genesis),
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

            EventType::Genesis => {
                let mut blockchain_guard = blockchain.lock().unwrap();
                blockchain_guard.fund_wallet(10000.00);
                info!("Genesis event");
                let hash_chain = HashChain::new();
                let hash_chain_message =  hash_chain.get_hash(EPOCH_DURATION as usize);
                info!("hash chain message: {:?}", hash_chain_message);
                // Create a stake transaction
                let wallet = &mut blockchain_guard.wallet;
                let public_key_str = wallet.get_public_key().to_string();
                let account = Account { address: public_key_str.clone() };
            
                // Create the transaction
                let stake_txn = Transaction::new(
                    wallet,
                    account.clone(),
                    account.clone(),
                    1000.00,
                    0,
                    TransactionType::STAKE
                ).unwrap();
                let genesis = Genesis::new(hash_chain_message, stake_txn.clone());
                let json = serde_json::to_string(&genesis).unwrap();
                info!("Serialized Genesis size: {} bytes", json.len());
                let serialized = bincode::serialize(&genesis).unwrap();
                info!("Serialized Genesis size: {} bytes", serialized.len());

                // Unlocked the blockchain
                drop(blockchain_guard);
                let test = swarm.behaviour_mut().floodsub.publish(p2p::GENESIS_TOPIC.clone(), serialized);
                info!("test: {:?}", test);
                // info!("New Epoch");
                // let hash_chain = HashChain::new();
                // let hash_chain_message =  hash_chain.get_hash(EPOCH_DURATION as usize);
                // let json = serde_json::to_string(&hash_chain_message).unwrap();
                // swarm.behaviour_mut().floodsub.publish(p2p::HASH_CHAIN_TOPIC.clone(), json.as_bytes());

            
            }

            EventType::Epoch => {
                info!("New Epoch");
                let hash_chain = HashChain::new();
                let hash_chain_message =  hash_chain.get_hash(EPOCH_DURATION as usize);
                let json = serde_json::to_string(&hash_chain_message).unwrap();
                swarm.behaviour_mut().floodsub.publish(p2p::HASH_CHAIN_TOPIC.clone(), json.as_bytes());
            }

            EventType::Mining => {
                info!("mining event");
                
                let mut blockchain = blockchain.lock().unwrap();
                if (blockchain.epoch.timestamp % EPOCH_DURATION) != 0 {
                let next_seed = blockchain.get_next_seed();
                let proposer = blockchain.select_block_proposer(next_seed);
                if proposer.address == blockchain.wallet.get_public_key().to_string() {
                    info!("I am the proposer for the new epoch");
                    // Pull the hash chain index for the new block
                    let hash_chain_index = blockchain.hash_chain.get_hash(blockchain.epoch.timestamp as usize);
                    // Propose the new block
                    let my_address = Account { address: blockchain.wallet.get_public_key().to_string() };
                    let new_block = blockchain.propose_block(
                        hash_chain_index.hash_chain_index, my_address, next_seed);
                    // Add the new block to the chain
                    blockchain.chain.push(new_block.clone());
                    // Execute the new block
                    blockchain.execute_block(new_block.clone());
                    // Broadcast the new block
                    let json = serde_json::to_string(&new_block).expect("Failed to serialize block");
                    swarm.behaviour_mut().floodsub.publish(p2p::BLOCK_TOPIC.clone(), json.as_bytes());
                    }
                }
            }
            EventType::HashChain => {
                info!("hash chain event");
                let hash_chain = HashChain::new();
                blockchain.lock().unwrap().hash_chain = hash_chain.clone();
                let json = serde_json::to_string(&hash_chain).unwrap();
                swarm.behaviour_mut().floodsub.publish(p2p::HASH_CHAIN_TOPIC.clone(), json.as_bytes());
            }   
        }
    }


     }

}
