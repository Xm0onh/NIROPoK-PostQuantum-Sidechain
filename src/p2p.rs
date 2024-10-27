
use crate::transaction::Transaction;
use crate::block::Block;
use crate::blockchain::Blockchain;

use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{tokio::Behaviour as Mdns, Event as MdnsEvent},
    swarm::{Swarm, NetworkBehaviour},
    PeerId,
};
use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy;
use serde_json;
use std::{collections::HashSet, sync::{Arc, Mutex}};
use log::info;


#[allow(dead_code)]
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from_public_key(&KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static TRANSACTION_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("transactions"));

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
    Init,
    Mining
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

        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(TRANSACTION_TOPIC.clone());
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
            let mut blockchain = blockchain.lock().unwrap();
            if let Ok(data) = serde_json::from_slice::<ChainResponse>(&message.data) {
                if data.from_peer_id ==  PEER_ID.to_string() {
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
        }
    }

    fn handle_mdns_event(&mut self, event: MdnsEvent) {
    }
}
