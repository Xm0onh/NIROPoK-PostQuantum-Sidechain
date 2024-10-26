
use crate::transaction::Transaction;
use crate::block::Block;


use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{tokio::Behaviour as Mdns, Event as MdnsEvent},
    swarm::{Swarm, NetworkBehaviour},
    PeerId,
};
use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy;

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
    pub from_peer_id: PeerId,
}


