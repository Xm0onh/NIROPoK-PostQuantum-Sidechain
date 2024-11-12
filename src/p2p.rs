use crate::transaction::Transaction;
use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::hashchain::HashChainMessage;
use crate::accounts::Account;
use crate::validator::Validator;
use crate::genesis::Genesis;
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{tokio::Behaviour as Mdns, Event as MdnsEvent},
    swarm::NetworkBehaviour,
    PeerId,
};
use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy;
use serde_json;
use std::sync::{Arc, Mutex};
use log::info;


#[allow(dead_code)]
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from_public_key(&KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static TRANSACTION_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("transactions"));
pub static HASH_CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("hash_chains"));
pub static GENESIS_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("genesis"));
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainRequest {
    pub from_peer_id: PeerId,
    
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub txns: Vec<Transaction>,
    pub from_peer_id: String,
}

#[allow(dead_code)]
pub enum EventType {
    Command(String),
    Genesis,
    Epoch,
    Mining,
    HashChain,
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "P2PEvent", prelude = "libp2p::swarm::derive_prelude")]
pub struct AppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
}

#[derive(Debug)]
pub enum P2PEvent {
    Floodsub(FloodsubEvent),
    Mdns(MdnsEvent),
}


impl From<FloodsubEvent> for P2PEvent {
    fn from(event: FloodsubEvent) -> Self {
        P2PEvent::Floodsub(event)
    }
}

impl From<MdnsEvent> for P2PEvent {
    fn from(event: MdnsEvent) -> Self {
        P2PEvent::Mdns(event)
    }
}


impl AppBehaviour {
    pub async fn new() -> Self {
        let mut behaviour = Self {
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default(), *PEER_ID).expect("Failed to create mDNS behaviour"),
        };

        info!("Subscribing to topics...");
        behaviour.floodsub.subscribe(GENESIS_TOPIC.clone());
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(TRANSACTION_TOPIC.clone());
        behaviour.floodsub.subscribe(HASH_CHAIN_TOPIC.clone());
        behaviour
    }

    pub fn handle_event(&mut self, event: P2PEvent, blockchain: Arc<Mutex<Blockchain>>) {
        match event {
            P2PEvent::Floodsub(event) => self.handle_floodsub_event(event, blockchain),
            P2PEvent::Mdns(event) => self.handle_mdns_event(event),
        }
    }

    fn handle_floodsub_event(&mut self, event: FloodsubEvent, blockchain: Arc<Mutex<Blockchain>>) {
        if let FloodsubEvent::Message(message) = event {
            info!("message: {:?}", message.data);

            let mut blockchain = blockchain.lock().unwrap();
            // Genesis message
            if let Ok(genesis) = bincode::deserialize::<Genesis>(&message.data) {
                info!("Received genesis message from {:?}", message.source);
                // Add the validator to the validator set
                let account = Account { address: genesis.stake_txn.recipient.address.clone() };
                blockchain.validator.add_validator(account, genesis.stake_txn.clone()).unwrap();
            }
            // Chain response
            else if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&message.data) {
                if resp.from_peer_id ==  PEER_ID.to_string() {
                    info!("Received chain from {:?}", message.source);
                    // blockchain.replace_chain(&data.blocks);
                    /*
                    blockchain.mempool.transactions = data
                    .txns
                    .into_iter()
                    .filter(|txn| Transaction::verify(txn).is_ok()) 
                    .collect();
                     */
                }
            } 
            // Chain Request
            else if let Ok(req) = serde_json::from_slice::<ChainRequest>(&message.data) {
                info!("Received chain request from {:?}", message.source);
                info!("Sending the chain and mempool to {:?}", message.source);
                let peer_id = req.from_peer_id;
                if peer_id == *PEER_ID {
                    // TODO: send the chain and mempool
                    // let json = serde_json::to_string(&ChainResponse{
                    //     blocks: blockchain.chain.clone(),
                    //     txns: blockchain.mempool.transactions.clone(),
                    //     from_peer_id: PEER_ID.to_string(),
                    // }).expect("Failed to serialize chain response");

                    // self.floodsub.publish(CHAIN_TOPIC.clone(), json.as_bytes())
                }
            }
            // Receive a new Transaction
            else if let Ok(txn) = serde_json::from_slice::<Transaction>(&message.data) {
                info!("Received a new transaction from {:?}", message.source);
                if txn.verify().unwrap() && !blockchain.mempool.txn_exists(&txn.hash) {
                    blockchain.mempool.add_transaction(txn.clone());
                    // relay the transaction to other peers
                    let json = serde_json::to_string(&txn).expect("Failed to serialize transaction");
                    self.floodsub.publish(TRANSACTION_TOPIC.clone(), json.as_bytes());
                }
            }

            // Receive a new Block
            else if let Ok(block) = serde_json::from_slice::<Block>(&message.data) {
                info!("Received a block from {:?}", message.source);
                if blockchain.verify_block(block.clone()).unwrap() && !blockchain.block_exists(block.clone()) {
                    // relay the block to other peers
                    let json = serde_json::to_string(&block).expect("Failed to serialize block");
                    self.floodsub.publish(BLOCK_TOPIC.clone(), json.as_bytes());
                    blockchain.execute_block(block.clone());

                    // Progress the epoch
                    blockchain.epoch.progress();

                    // Check if it is the end of the epoch
                    if blockchain.epoch.is_end_of_epoch() {
                        blockchain.end_of_epoch();
                    }
                }
            }
            // Hash chain message
            else if let Ok(msg) = serde_json::from_slice::<HashChainMessage>(&message.data) {
                info!("Received a hash chain message from {:?}: {:?}", message.source, msg);
                Validator::update_validator_com(&mut blockchain.validator, Account { address: message.source.to_string() }  , msg);
                // Check if it received the hash chain from all validators
                info!("Checking if the hash chain is complete");
                if blockchain.validator.hash_chain_received() {
                    let seed = blockchain.new_epoch();
                    let proposer = blockchain.select_block_proposer(seed);
                    if proposer.address == blockchain.wallet.get_public_key().to_string() {
                        info!("I am the proposer for the new epoch");
                        blockchain.epoch.progress();
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
                            self.floodsub.publish(BLOCK_TOPIC.clone(), json.as_bytes());
                        }
                    }
                }
            }
            
            // Simple String
            else if let Ok(msg) = serde_json::from_slice::<String>(&message.data) {
                info!("Received a simple string from {:?}: {:?}", message.source, msg);
            } else {
                info!("Received an unknown message from {:?}: {:?}", message.source, message.data);
            }
        
        }
    }

    fn handle_mdns_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer_id, addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer_id);
                    info!("Discovered new peer: {:?}, addr: {:?}", peer_id, addr);
                }
        }
         MdnsEvent::Expired(expired_list) => {
            for (peer_id, addr) in expired_list {
                self.floodsub.remove_node_from_partial_view(&peer_id);
                info!("Expired peer: {:?}, addr: {:?}", peer_id, addr);
            }
         }
    }

    }
}
