use crate::accounts::Account;
use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::genesis::Genesis;
use crate::hashchain::{verify_hash_chain_index, HashChainCom, HashChainMessage};
use crate::transaction::Transaction;
use crate::validator::Validator;
use libp2p::{
    gossipsub::{
        Behaviour, ConfigBuilder, Event, IdentTopic as Topic, MessageAuthenticity, PeerScoreParams,
        PeerScoreThresholds,
    },
    identity,
    mdns::{tokio::Behaviour as Mdns, Event as MdnsEvent},
    swarm::NetworkBehaviour,
    PeerId,
};
use log::error;

use hex;
use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::{Arc, Mutex};

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from_public_key(&KEYS.public()));

pub static GENESIS_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("genesis"));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static TRANSACTION_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("transactions"));
pub static HASH_CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("hash_chains"));
pub static HASH_CHAIN_MESSAGE_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("hash_chain_messages"));
pub static BLOCK_SIGNATURE_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("block_signatures"));

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
    RpcTransaction(Transaction),
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "P2PEvent")]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockSignature {
    pub block_id: usize,
    pub block_hash: String,
    pub sender: Account,
    pub signature: Vec<u8>,
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

        let mut gossipsub =
            Behaviour::new(MessageAuthenticity::Signed(KEYS.clone()), gossipsub_config)
                .expect("Failed to create Gossipsub behaviour with peer scoring");

        gossipsub
            .with_peer_score(peer_score_params, peer_score_thresholds)
            .expect("Failed to set peer scoring");

        let mut behaviour = Self {
            gossipsub,
            mdns: Mdns::new(Default::default(), *PEER_ID).expect("Failed to create mDNS behaviour"),
        };

        info!("Subscribing to topics...");
        behaviour.gossipsub.subscribe(&GENESIS_TOPIC).unwrap();
        behaviour.gossipsub.subscribe(&CHAIN_TOPIC).unwrap();
        behaviour.gossipsub.subscribe(&BLOCK_TOPIC).unwrap();
        behaviour
            .gossipsub
            .subscribe(&BLOCK_SIGNATURE_TOPIC)
            .unwrap();
        behaviour.gossipsub.subscribe(&TRANSACTION_TOPIC).unwrap();
        behaviour.gossipsub.subscribe(&HASH_CHAIN_TOPIC).unwrap();
        behaviour
            .gossipsub
            .subscribe(&HASH_CHAIN_MESSAGE_TOPIC)
            .unwrap();
        behaviour
    }
    pub fn handle_event(&mut self, event: P2PEvent, blockchain: Arc<Mutex<Blockchain>>) {
        match event {
            P2PEvent::Gossipsub(event) => self.handle_gossipsub_event(event, blockchain),
            P2PEvent::Mdns(event) => self.handle_mdns_event(event),
        }
    }

    fn handle_gossipsub_event(&mut self, event: Event, blockchain: Arc<Mutex<Blockchain>>) {
        match event {
            Event::Message {
                propagation_source,
                message_id: _,
                message,
            } => {
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
            info!(
                "Added validator {:?}, with stake {:?}",
                result, genesis.stake_txn.amount
            );
            warn!(
                "Size of validator: {:?}",
                blockchain.validator.state.accounts.len()
            );
        } else if let Ok(resp) = serde_json::from_slice::<ChainResponse>(data) {
            if resp.from_peer_id == PEER_ID.to_string() {
                info!("Received chain from {:?}", source);
                // Handle the ChainResponse
            }
        } else if let Ok(req) = serde_json::from_slice::<ChainRequest>(data) {
            info!("Received chain request from {:?}", source);
            info!("Sending the chain and mempool to {:?}", source);
            let peer_id = req.from_peer_id;
            if peer_id == *PEER_ID {
                // TODO: send the chain and mempool
            }

        // Receive a Transaction
        } else if let Ok(txn) = serde_json::from_slice::<Transaction>(data) {
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
        // Receive a Block
        else if let Ok(block) = serde_json::from_slice::<Block>(data) {
            info!("Received a block from {:?}", source);
            if blockchain.verify_block(block.clone()) {
                if !blockchain.block_exists(block.clone()) {
                    blockchain.execute_block(block.clone());
                    info!("Executed block {:?}", block.id);
                    // Progress the epoch once when executing a new block
                    blockchain.epoch.progress();
                }

                // NEW: Ensure every node signs if it hasn't already
                {
                    let local_pub = blockchain.wallet.get_public_key().to_string();
                    // Check if this node already signed the block
                    let already_signed = blockchain
                        .pending_signatures
                        .get(&block.id)
                        .map(|sigs| sigs.iter().any(|s| s.sender.address == local_pub))
                        .unwrap_or(false);
                    if !already_signed {
                        let block_hash_hex = hex::encode(&block.hash);
                        let signature = blockchain.wallet.sign_message(block_hash_hex.as_bytes());
                        let block_sig = BlockSignature {
                            block_id: block.id,
                            block_hash: block_hash_hex,
                            sender: Account { address: local_pub },
                            signature: signature.to_vec(),
                        };
                        let json = serde_json::to_string(&block_sig).unwrap();
                        self.gossipsub
                            .publish(BLOCK_SIGNATURE_TOPIC.clone(), json.into_bytes())
                            .unwrap();
                    }
                }
            } else {
                info!("Block failed verification from {:?}", source);
            }

            // Check if it is the end of the epoch
            if blockchain.epoch.is_end_of_epoch() {
                blockchain.end_of_epoch();
            }
        }
        // Receive a HashChainCom - Commitment for the epoch
        else if let Ok(msg) = serde_json::from_slice::<HashChainCom>(data) {
            Validator::update_validator_com(
                &mut blockchain.validator,
                Account {
                    address: msg.sender.address.clone(),
                },
                msg.clone(),
            );
            info!("Receivedrom {:?}", msg.hash_chain_index);
        }
        // Receive a HashChainMessage - HashChainMessage is the message that contains the hash of the hash chain
        else if let Ok(msg) = serde_json::from_slice::<HashChainMessage>(data) {
            let validator_commitment = blockchain
                .validator
                .get_validator_commitment(msg.sender.clone());
            let received_commitment = msg.hash.clone();
            if verify_hash_chain_index(
                validator_commitment.hash_chain_index.clone(),
                msg.epoch as u64,
                received_commitment.clone(),
            ) {
                info!("Received valid hash chain message");
                blockchain
                    .validator
                    .next_block_hash
                    .insert(msg.sender.clone(), msg.hash.clone());
            } else {
                error!("Received invalid hash chain message from");
                error!(
                    "Validator commitment: {}",
                    validator_commitment.hash_chain_index
                );
                error!("Received commitment: {}", received_commitment);
            }
        }
        // NEW: Process BlockSignature messages - only relevant for the block producer
        else if let Ok(block_sig) = serde_json::from_slice::<BlockSignature>(data) {
            info!(
                "Received block signature for block {} from {:?}",
                block_sig.block_id, source
            );
            // Let the blockchain (if this node is the block producer) collect the signature
            blockchain.collect_block_signature(block_sig);
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
