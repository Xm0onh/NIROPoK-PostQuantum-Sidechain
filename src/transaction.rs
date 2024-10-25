use serde::{Deserialize, Serialize};
use chrono::Utc;


#[derive(Debug, Serialize, Deserialize)]
pub enum TransactionType {
    TRANSACTION,
    COINBASE,
    STAKE,
    UNSTAKE,
    DELEGATE,
    RESIGN,
    VALIDATOR,
    ValidatorReward,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub sender: String,
    pub recipient: String,
    pub signature: String,
    pub amount: usize,
    pub timestamp: usize,
    pub fee: usize,
    pub txn_type: TransactionType,
}



#[allow(dead_code)]
impl Transaction {
    // TODO - sender/recipient should be Wallet struct
    // TODO - signature should be generated based on CRYSTAL-Dilithium
    pub fn new(
        sender: String, 
        recipient: String, 
        amount: usize, 
        _timestamp: usize,
        fee: usize, 
        txn_type: TransactionType) -> Result<Self, String> {
        Ok(Self {
                // TODO - hash should be generated based on all fields
                hash: String::new(),
                sender,
                recipient,
                signature: String::new(),
                amount,
                timestamp: Utc::now().timestamp_millis() as usize,
                fee,
                txn_type})
    }
    // TODO - verify transaction
    pub fn verify(&self) -> Result<bool, String> {
        Ok(true)
    }
}
