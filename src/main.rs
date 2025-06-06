use colored::*;
use futures::stream::StreamExt;
use libp2p::{
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr,
};
use p2p::EventType;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::{sleep, interval},
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
mod networking;
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
use crate::utils::TpsTracker;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting the new Peer, {}", p2p::PEER_ID.clone());
    let (epoch_sender, mut epoch_rcv) = mpsc::unbounded_channel::<bool>();
    let (mining_sender, mut mining_rcv) = mpsc::unbounded_channel::<bool>();
    let (genesis_sender, mut genesis_rcv) = mpsc::unbounded_channel::<bool>();
    let (rpc_sender, mut rpc_rcv) = mpsc::unbounded_channel::<Transaction>();

    let wallet = wallet::Wallet::new().unwrap();
    let blockchain = Arc::new(Mutex::new(Blockchain::new(wallet)));

    // --- Initialize TPS Tracker ---
    let tps_tracker = Arc::new(Mutex::new(TpsTracker {
        start_time: Instant::now(),
        total_transactions_confirmed: 0,
    }));
    // --- End Initialize TPS Tracker ---

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

    // Spawn RPC server to receive transactions via HTTP POST requests
    let rpc_sender_clone = rpc_sender.clone();
    tokio::spawn(async move {
        networking::start_rpc_server(rpc_sender_clone).await;
    });

    // --- Add this block for TPS reporting ---
    let tps_tracker_clone_reporter = Arc::clone(&tps_tracker);
    tokio::spawn(async move {
        let mut report_interval = interval(Duration::from_secs(10)); // Report every 10 seconds
        loop {
            report_interval.tick().await;
            let tracker = tps_tracker_clone_reporter.lock().unwrap();
            let elapsed = tracker.start_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let tps = tracker.total_transactions_confirmed as f64 / elapsed;
                info!(
                    "[{:.2}s elapsed] Total Confirmed Txns: {}, Average TPS: {:.2}",
                    elapsed, tracker.total_transactions_confirmed, tps
                );
            }
        }
    });
    // --- End TPS reporting block ---

    loop {
        let evt = {
            select! {
                line = stdin.next_line() => Some(p2p::EventType::Command(line.expect("can get line").expect("can read line from stdin"))),
                _epoch = epoch_rcv.recv() => Some(p2p::EventType::Epoch),
                _mining = mining_rcv.recv() => Some(p2p::EventType::Mining),
                _genesis = genesis_rcv.recv() => Some(p2p::EventType::Genesis),
                rpc = rpc_rcv.recv() => rpc.map(|txn| p2p::EventType::RpcTransaction(txn)),
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(e) => {
                            let behaviour = swarm.behaviour_mut();
                            behaviour.handle_event(e, Arc::clone(&blockchain), Arc::clone(&tps_tracker));
                            None
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Listening on {:?}", address);
                            None
                        }
                        _ => None
                    }
                }
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

                    let mut blockchain_guard = blockchain.lock().unwrap();
                    info!("Epoch: {}", blockchain_guard.epoch.timestamp);

                    let next_seed: Option<Seed>;

                    if blockchain_guard.epoch.timestamp == 1 {
                        next_seed = Some(blockchain_guard.new_epoch());
                    } else if (blockchain_guard.epoch.timestamp % EPOCH_DURATION) != 0 {
                        next_seed = Some(blockchain_guard.get_next_seed());
                    } else if blockchain_guard.epoch.is_end_of_epoch() || blockchain_guard.epoch.timestamp == 0
                    {
                        info!("End of Epoch");
                        blockchain_guard.end_of_epoch();
                        let epoch_sender_clone = epoch_sender.clone();
                        epoch_sender_clone
                            .send(true)
                            .expect("can't send epoch event");
                        next_seed = None;
                    } else {
                         next_seed = None;
                         log::warn!("Unexpected state in mining event loop");
                    }

                    if let Some(seed) = next_seed {
                        let tps_tracker_clone_miner = Arc::clone(&tps_tracker);
                        handle_block_proposal(&mut blockchain_guard, seed, &mut swarm, tps_tracker_clone_miner);
                    }
                }

                EventType::RpcTransaction(txn) => {
                    info!("Received RPC transaction: {:?}", txn);
                    let mut blockchain_guard = blockchain.lock().unwrap();
                    if txn.verify().unwrap() && !blockchain_guard.mempool.txn_exists(&txn.hash) {
                        blockchain_guard.mempool.add_transaction(txn.clone());
                        let json = serde_json::to_string(&txn)
                            .expect("Failed to serialize RPC transaction");
                        swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(p2p::TRANSACTION_TOPIC.clone(), json.into_bytes())
                            .unwrap();
                        info!("RPC transaction processed and relayed: {:?}", txn.hash);
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
        tps_tracker: Arc<Mutex<TpsTracker>>,
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
            let hash_chain_index = blockchain.hash_chain.get_hash(
                EPOCH_DURATION as usize - blockchain.epoch.timestamp as usize + 1,
                proposer.clone(),
            );
            // --- Fetch Transactions from Mempool ---
            let txns_to_include = blockchain.mempool.get_transactions(MAX_TXNS_PER_BLOCK);
            // --- End Fetch Transactions ---
            let my_address = Account {
                address: blockchain.wallet.get_public_key().to_string(),
            };
            let new_block = blockchain.propose_block(
                hash_chain_index.hash_chain_index,
                my_address,
                txns_to_include,
                seed);
            let confirmed_txns_count = new_block.txn.len() as u64;
            blockchain.execute_block(new_block.clone());

            if confirmed_txns_count > 0 {
                let mut tracker = tps_tracker.lock().unwrap();
                tracker.total_transactions_confirmed += confirmed_txns_count;
            }

            blockchain.epoch.progress();
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
