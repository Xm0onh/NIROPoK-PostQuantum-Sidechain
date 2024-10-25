use serde::{Deserialize, Serialize};
use sha2::digest_bytes;


pub struct Block {
    pub id: usize,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: usize,
    pub txn: Vec<Transaction>,
}


pub fn calculate_hash(
    id: &usize,
    timestamp: &i64,
    previous_hash: &String,
    txn: &Vec<Transaction>,
) -> String {
     info!("Calculating hash for block: {:?}", id);
     let hash = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "timestamp": timestamp,
        "txns": txn,
     });

     Util::hash(&hash.to_string())
}

pub fn hash(s: &str) -> String {
    digest_bytes(s.as_bytes())
}

pub fn genesis(wallet: Wallet) -> Block {
    ifno!("Creating genesis block");
    Block::new(0, String::from("genesis"), String::from("genesis"), 0, Vec::new(), Wallet)
}