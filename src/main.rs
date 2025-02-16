use colored::*;
use futures::stream::StreamExt;
use libp2p::{
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr,
};
use p2p::EventType;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};

mod accounts;
mod block;
mod blockchain;
mod ccok;
mod config;
mod epoch;
mod genesis;
mod hashchain;
mod mempool;
mod merkle;
mod p2p;
mod transaction;
mod utils;
mod validator;
mod wallet;

use accounts::Account;
use blockchain::Blockchain;
use config::*;
use genesis::Genesis;
use hashchain::HashChain;
use hashchain::HashChainCom;
use log::info;
use transaction::{Transaction, TransactionType};
use utils::Seed;
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
    let transport =
        libp2p::tokio_development_transport(p2p::KEYS.clone()).expect("Failed to create transport");

    let mut swarm =
        SwarmBuilder::with_tokio_executor(transport, behavior, p2p::PEER_ID.clone()).build();

    let mut stdin: tokio::io::Lines<BufReader<tokio::io::Stdin>> = BufReader::new(stdin()).lines();

    let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0"
        .parse()
        .expect("Failed to parse listen address");

    swarm
        .listen_on(listen_addr)
        .expect("Failed to listen on address");

    // Genesis event is just a simple event for registering the first nodes and update the state for their stake value - it should change in the future
    let genesis_sender_clone = genesis_sender.clone();
    spawn(async move {
        sleep(Duration::from_secs(5)).await;
        info!("sending genesis event");
        genesis_sender_clone
            .send(true)
            .expect("can't send genesis event");
    });

    let epoch_sender_clone = epoch_sender.clone();
    spawn(async move {
        sleep(Duration::from_secs(10)).await;
        info!("sending epoch event");
        epoch_sender_clone
            .send(true)
            .expect("can't send epoch event");
    });

    let mut planner = periodic::Planner::new();
    planner.start();
    spawn(async move {
        sleep(Duration::from_secs(15)).await;
        info!("sending mining event");
        let mining_sender_clone = mining_sender.clone();
        planner.add(
            move || {
                mining_sender_clone
                    .send(true)
                    .expect("can't send mining event")
            },
            periodic::Every::new(Duration::from_secs(BLOCK_INTERVAL)),
        );
    });

    loop {
        let evt = {
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
                    let account = Account {
                        address: public_key_str.clone(),
                    };
                    // Create the transaction
                    let stake_txn = Transaction::new(
                        wallet,
                        account.clone(),
                        account.clone(),
                        100.00,
                        0,
                        TransactionType::STAKE,
                    )
                    .unwrap();
                    let genesis = Genesis::new(stake_txn.clone());
                    let json = serde_json::to_string(&genesis).unwrap();
                    info!("Serialized Genesis size: {} bytes", json.len());
                    let serialized = bincode::serialize(&genesis).unwrap();
                    info!("Serialized Genesis size: {} bytes", serialized.len());
                    drop(blockchain_guard);
                    let test = swarm
                        .behaviour_mut()
                        .gossipsub
                        .publish(p2p::GENESIS_TOPIC.clone(), serialized);
                    info!("test: {:?}", test);
                }

                EventType::Epoch => {
                    info!("New Epoch");
                    let mut blockchain: std::sync::MutexGuard<'_, Blockchain> =
                        blockchain.lock().unwrap();
                    let my_address = Account {
                        address: blockchain.wallet.get_public_key().to_string(),
                    };
                    let hash_chain = HashChain::new();

                    // Commitment is the last hash in the hash chain
                    let commitment = hash_chain.hash_chain.last().unwrap();
                    let hash_chain_message = HashChainCom {
                        hash_chain_index: commitment.clone(),
                        sender: my_address.clone(),
                    };

                    blockchain
                        .validator
                        .update_validator_com(my_address.clone(), hash_chain_message.clone());
                    let json = serde_json::to_string(&hash_chain_message).unwrap();
                    swarm
                        .behaviour_mut()
                        .gossipsub
                        .publish(p2p::HASH_CHAIN_TOPIC.clone(), json.as_bytes())
                        .unwrap();
                    blockchain.epoch.progress();
                    blockchain.hash_chain = hash_chain.clone();
                    info!("Epoch: {}", blockchain.epoch.timestamp);
                    drop(blockchain);
                }

                EventType::Mining => {
                    let peer_count = swarm.behaviour().gossipsub.all_peers().count();
                    if peer_count < 3 {
                        info!("Not enough nodes connected for block production, current peer count: {}", peer_count);
                        continue;
                    }
                    info!("mining event");

                    let mut blockchain = blockchain.lock().unwrap();
                    info!("Epoch: {}", blockchain.epoch.timestamp);

                    if blockchain.epoch.timestamp == 1 {
                        let new_epoch = blockchain.new_epoch();
                        handle_block_proposal(&mut blockchain, new_epoch, &mut swarm);
                    } else if (blockchain.epoch.timestamp % EPOCH_DURATION) != 0 {
                        let next_seed = blockchain.get_next_seed();
                        handle_block_proposal(&mut blockchain, next_seed, &mut swarm);
                    } else if blockchain.epoch.is_end_of_epoch() || blockchain.epoch.timestamp == 0
                    {
                        info!("End of Epoch");
                        blockchain.end_of_epoch();
                        let epoch_sender_clone = epoch_sender.clone();
                        epoch_sender_clone
                            .send(true)
                            .expect("can't send epoch event");
                    }
                }

                EventType::HashChain => {}
            }
        }
    }
    // Handle the block proposal
    pub fn handle_block_proposal(
        blockchain: &mut Blockchain,
        seed: Seed,
        swarm: &mut libp2p::Swarm<p2p::AppBehaviour>,
    ) {
        let proposer = blockchain.select_block_proposer(seed);
        if proposer.address == blockchain.wallet.get_public_key().to_string() {
            info!(
                "{}",
                format!(
                    "👷 I am the proposer for the new block {:?}",
                    blockchain.get_latest_block_id() + 1
                )
                .bright_green()
            );
            // Pull the hash chain index for the new block
            let hash_chain_index = blockchain.hash_chain.get_hash(
                EPOCH_DURATION as usize - blockchain.epoch.timestamp as usize + 1,
                proposer.clone(),
            );
            let txns = vec![];
            // Propose the new block
            let my_address = Account {
                address: blockchain.wallet.get_public_key().to_string(),
            };
            let new_block =
                blockchain.propose_block(hash_chain_index.hash_chain_index, my_address, txns, seed);
            // Execute the new block
            blockchain.execute_block(new_block.clone());
            blockchain.epoch.progress();
            // Broadcast the new block
            let json = serde_json::to_string(&new_block).expect("Failed to serialize block");
            swarm
                .behaviour_mut()
                .gossipsub
                .publish(p2p::BLOCK_TOPIC.clone(), json.as_bytes())
                .unwrap();

            info!("📩 Block proposed: {:?}", new_block.id);
        }
    }
}
