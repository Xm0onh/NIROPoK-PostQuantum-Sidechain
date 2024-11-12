use crate::hashchain::HashChainMessage;
use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Genesis {
    pub hash_chain_com: HashChainMessage,
    pub stake_txn: Transaction,
}

impl Genesis {
    pub fn new(hash_chain_com: HashChainMessage, stake_txn: Transaction) -> Self {
        Self { hash_chain_com, stake_txn }
    }
}