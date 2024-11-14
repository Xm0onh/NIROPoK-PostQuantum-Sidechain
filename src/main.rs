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
use hashchain::HashChainMessage;
use config::*;
use transaction::{Transaction, TransactionType};
use log::{info};
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
     let epoch_sender_clone = epoch_sender.clone();
     spawn(async move {
         sleep(Duration::from_secs(10)).await;
         info!("sending epoch event");
         epoch_sender_clone.send(true).expect("can't send epoch event");
     });

    let mut planner = periodic::Planner::new();
    planner.start();
    spawn(async move {
        sleep(Duration::from_secs(15)).await;
        info!("sending mining event");
        let mining_sender_clone = mining_sender.clone();
        planner.add(
        move || mining_sender_clone.send(true).expect("can't send mining event"),
        periodic::Every::new(Duration::from_secs(BLOCK_INTERVAL)),
        );
    });
   
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
                    let behaviour = swarm.behaviour_mut();
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
                info!("Genesis event");
                // Create a stake transaction
                let wallet = &mut blockchain_guard.wallet;
                let public_key_str = wallet.get_public_key().to_string();
                let account = Account { address: public_key_str.clone() };
                // Create the transaction
                let stake_txn = Transaction::new(
                    wallet,
                    account.clone(),
                    account.clone(),
                    100.00,
                    0,
                    TransactionType::STAKE
                ).unwrap();
                let genesis = Genesis::new( stake_txn.clone());
                let json = serde_json::to_string(&genesis).unwrap();
                info!("Serialized Genesis size: {} bytes", json.len());
                let serialized = bincode::serialize(&genesis).unwrap();
                info!("Serialized Genesis size: {} bytes", serialized.len());
                drop(blockchain_guard);
                let test = swarm.behaviour_mut().gossipsub.publish(p2p::GENESIS_TOPIC.clone(), serialized);
                info!("test: {:?}", test);
            }

            EventType::Epoch => {
                info!("New Epoch");
                let mut blockchain: std::sync::MutexGuard<'_, Blockchain> = blockchain.lock().unwrap();
                let my_address = Account { address: blockchain.wallet.get_public_key().to_string() };
                let hash_chain = HashChain::new();

                // Commitment is the last hash in the hash chain
                let commitment = hash_chain.hash_chain.last().unwrap();
                let hash_chain_message = HashChainMessage {
                    sender: my_address.clone(),
                    hash_chain_index: commitment.clone()
                };
                blockchain.validator.update_validator_com(my_address.clone(), hash_chain_message.clone());
                let json = serde_json::to_string(&hash_chain_message).unwrap();
                swarm.behaviour_mut().gossipsub.publish(p2p::HASH_CHAIN_TOPIC.clone(), json.as_bytes()).unwrap();
                blockchain.epoch.progress();
                blockchain.hash_chain = hash_chain.clone();
                info!("Epoch: {}", blockchain.epoch.timestamp);
                drop(blockchain);
            }

            EventType::Mining => {
                info!("mining event");
                
                let mut blockchain = blockchain.lock().unwrap();
                info!("Epoch: {}", blockchain.epoch.timestamp);
                if blockchain.epoch.timestamp == 1 {
                    let new_epoch = blockchain.new_epoch();
                    let proposer = blockchain.select_block_proposer(new_epoch);
                    if proposer.address == blockchain.wallet.get_public_key().to_string() {
                        info!("I am the proposer for the new block {:?}", blockchain.epoch.timestamp);
                        // Pull the hash chain index for the new block
                        let hash_chain_index = blockchain.hash_chain.get_hash(
                            EPOCH_DURATION as usize - blockchain.epoch.timestamp as usize + 1, 
                            proposer.clone()
                        );
                        let txns = vec![];
                        // Propose the new block
                        let my_address = Account { address: blockchain.wallet.get_public_key().to_string() };
                        let new_block = blockchain.propose_block(hash_chain_index.hash_chain_index, my_address, txns, new_epoch);
                        // Execute the new block
                        blockchain.execute_block(new_block.clone());
                        blockchain.epoch.progress();
                        // Broadcast the new block
                        let json = serde_json::to_string(&new_block).expect("Failed to serialize block");
                        swarm.behaviour_mut().gossipsub.publish(p2p::BLOCK_TOPIC.clone(), json.as_bytes()).unwrap();
                    }

                }
                else if (blockchain.epoch.timestamp % EPOCH_DURATION) != 0 {
                    let next_seed = blockchain.get_next_seed();
                    let proposer = blockchain.select_block_proposer(next_seed);
                    if proposer.address == blockchain.wallet.get_public_key().to_string() {
                        let latest_block = blockchain.chain.last().unwrap();
                        info!("I am the proposer for the new block index {:?}", latest_block.id + 1);
                        // Pull the hash chain index for the new block
                        let hash_chain_index = blockchain.hash_chain.get_hash(
                            EPOCH_DURATION as usize - blockchain.epoch.timestamp as usize + 1, 
                            proposer.clone()
                        );
                        // Propose the new block
                        let my_address = Account { address: blockchain.wallet.get_public_key().to_string() };
                        let txns = vec![];
                        let new_block = blockchain.propose_block(
                            hash_chain_index.hash_chain_index, my_address, txns, next_seed);
                        // Add the new block to the chain
                        blockchain.chain.push(new_block.clone());
                        // Execute the new block
                        blockchain.execute_block(new_block.clone());
                        blockchain.epoch.progress();
                        if blockchain.epoch.is_end_of_epoch() {
                            blockchain.end_of_epoch();
                        }
                        // Broadcast the new block
                        let json = serde_json::to_string(&new_block).expect("Failed to serialize block");
                        swarm.behaviour_mut().gossipsub.publish(p2p::BLOCK_TOPIC.clone(), json.as_bytes()).unwrap();
                    }
                }
                else if blockchain.epoch.is_end_of_epoch() || blockchain.epoch.timestamp == 0 {
                    info!("End of Epoch");
                    blockchain.end_of_epoch();
                    let epoch_sender_clone = epoch_sender.clone();
                    epoch_sender_clone.send(true).expect("can't send epoch event");
                }
            }
            EventType::HashChain => {
                
            }   
        }
    }


     }

}
