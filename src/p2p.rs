use crate::transaction::Transaction;
use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::hashchain::HashChainMessage;
use crate::accounts::Account;
use crate::validator::Validator;
use crate::genesis::Genesis;
use libp2p::{
    gossipsub::{
        Behaviour,
        ConfigBuilder,
        PeerScoreParams,
        PeerScoreThresholds,
        Event,
        IdentTopic as Topic,
        MessageAuthenticity,
        MessageId,
    },
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

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from_public_key(&KEYS.public()));

pub static GENESIS_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("genesis"));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static TRANSACTION_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("transactions"));
pub static HASH_CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("hash_chains"));

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
#[behaviour(out_event = "P2PEvent")]
pub struct AppBehaviour {
    pub gossipsub: Behaviour,
    pub mdns: Mdns,
}

#[derive(Debug)]
pub enum P2PEvent {
    Gossipsub(Event),
    Mdns(MdnsEvent),
}

impl From<Event> for P2PEvent {
    fn from(event: Event) -> Self {
        P2PEvent::Gossipsub(event)
    }
}

impl From<MdnsEvent> for P2PEvent {
    fn from(event: MdnsEvent) -> Self {
        P2PEvent::Mdns(event)
    }
}

impl AppBehaviour {
    pub async fn new() -> Self {
        let gossipsub_config = ConfigBuilder::default()
            .mesh_outbound_min(1)
            .mesh_n_low(1)
            .mesh_n(2)
            .mesh_n_high(3)
            .flood_publish(true)
            .allow_self_origin(true)
            .max_transmit_size(16 * 1024 * 1024)
            .build()
            .expect("Valid Gossipsub config");
        // Create default peer score parameters
        let peer_score_params = PeerScoreParams::default();

        // Adjust peer score thresholds to accept all peers
        let peer_score_thresholds = PeerScoreThresholds {
            gossip_threshold: f64::MIN,
            publish_threshold: f64::MIN,
            graylist_threshold: f64::MIN,
            ..Default::default()
        };

        let mut gossipsub = Behaviour::new(
            MessageAuthenticity::Signed(KEYS.clone()),
            gossipsub_config,
        )
        .expect("Failed to create Gossipsub behaviour with peer scoring");
        
        gossipsub.with_peer_score(peer_score_params, peer_score_thresholds).expect("Failed to set peer scoring");

        let mut behaviour = Self {
            gossipsub,
            mdns: Mdns::new(Default::default(), *PEER_ID)
                .expect("Failed to create mDNS behaviour"),
        };

        info!("Subscribing to topics...");
        behaviour.gossipsub.subscribe(&GENESIS_TOPIC).unwrap();
        behaviour.gossipsub.subscribe(&CHAIN_TOPIC).unwrap();
        behaviour.gossipsub.subscribe(&BLOCK_TOPIC).unwrap();
        behaviour.gossipsub.subscribe(&TRANSACTION_TOPIC).unwrap();
        behaviour.gossipsub.subscribe(&HASH_CHAIN_TOPIC).unwrap();

        behaviour
    }
    pub fn handle_event(&mut self, event: P2PEvent, blockchain: Arc<Mutex<Blockchain>>) {
        match event {
            P2PEvent::Gossipsub(event) => self.handle_gossipsub_event(event, blockchain),
            P2PEvent::Mdns(event) => self.handle_mdns_event(event),
        }
    }

    fn handle_gossipsub_event(
        &mut self,
        event: Event,
        blockchain: Arc<Mutex<Blockchain>>,
    ) {
        match event {
            Event::Message { propagation_source, message_id: _, message } => {
                let data = &message.data;
                let source = message.source.unwrap_or(propagation_source);
                self.process_message(data, source, blockchain);
            }
            _ => {}
        }
    }
    

    fn process_message(&mut self, data: &[u8], source: PeerId, blockchain: Arc<Mutex<Blockchain>>) {
        let mut blockchain = blockchain.lock().unwrap();

        if let Ok(genesis) = bincode::deserialize::<Genesis>(data) {
            info!("Received genesis message from {:?}", source);
            let account = Account {
                address: genesis.stake_txn.recipient.address.clone(),
            };
            let result = blockchain
                .validator
                .add_validator(account.clone(), genesis.stake_txn.clone())
                .unwrap();
            info!("Added validator {:?}", result);
        }
        else if let Ok(resp) = serde_json::from_slice::<ChainResponse>(data) {
            if resp.from_peer_id == PEER_ID.to_string() {
                info!("Received chain from {:?}", source);
                // Handle the ChainResponse
            }
        }
        else if let Ok(req) = serde_json::from_slice::<ChainRequest>(data) {
            info!("Received chain request from {:?}", source);
            info!("Sending the chain and mempool to {:?}", source);
            let peer_id = req.from_peer_id;
            if peer_id == *PEER_ID {
                // TODO: send the chain and mempool
            }
        }
        else if let Ok(txn) = serde_json::from_slice::<Transaction>(data) {
            info!("Received a new transaction from {:?}", source);
            if txn.verify().unwrap() && !blockchain.mempool.txn_exists(&txn.hash) {
                blockchain.mempool.add_transaction(txn.clone());
                // Relay the transaction to other peers
                let json = serde_json::to_string(&txn).expect("Failed to serialize transaction");
                if let Err(e) = self
                    .gossipsub
                    .publish(TRANSACTION_TOPIC.clone(), json.into_bytes())
                {
                    eprintln!("Failed to publish transaction: {}", e);
                }
            }
        }
        // Try deserializing as Block
        else if let Ok(block) = serde_json::from_slice::<Block>(data) {
            info!("Received a block from {:?}", source);
            if blockchain.verify_block(block.clone()).unwrap() && !blockchain.block_exists(block.clone()) {
                // Relay the block to other peers
                let json = serde_json::to_string(&block).expect("Failed to serialize block");
                if let Err(e) = self.gossipsub.publish(BLOCK_TOPIC.clone(), json.into_bytes()) {
                    eprintln!("Failed to publish block: {}", e);
                }
                blockchain.execute_block(block.clone());
                info!("Executed block {:?}", block.id);
                // Progress the epoch
                blockchain.epoch.progress();

                // Check if it is the end of the epoch
                if blockchain.epoch.is_end_of_epoch() {
                    blockchain.end_of_epoch();
                }
            } else if blockchain.block_exists(block.clone()) {
                info!("Block {:?} already exists", block.id);
            }
        }
        // Try deserializing as HashChainMessage
        else if let Ok(msg) = serde_json::from_slice::<HashChainMessage>(data) {
            info!("Received a hash chain message from {:?}: {:?}", source, msg);
            Validator::update_validator_com(
                &mut blockchain.validator,
                Account {
                    address: source.to_string(),
                },
                msg,
            );
            // check the commitment
            let commitment = blockchain.validator.get_validator_commitment(Account {
                address: source.to_string(),
            });
            info!("Commitment: {:?}", commitment);
            info!("Account: {:?}", Account {
                address: source.to_string(),
            });
           
        }
        // Try deserializing as String
        else if let Ok(msg) = serde_json::from_slice::<String>(data) {
            // Start Mining
        } else {
            info!("Received an unknown message from {:?}: {:?}", source, data);
        }
    }

    fn handle_mdns_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer_id, addr) in discovered_list {
                    self.gossipsub.add_explicit_peer(&peer_id); // Gossipsub handles peer connections automatically
                    info!("Discovered new peer: {:?}, addr: {:?}", peer_id, addr);

                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer_id, _) in expired_list {
                    self.gossipsub.remove_explicit_peer(&peer_id);
                    info!("Expired peer: {:?}", peer_id);
                }
            }
        }
    }
}
