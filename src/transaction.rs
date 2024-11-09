use serde::{Deserialize, Serialize};
use crystals_dilithium::dilithium2::PublicKey;
use sha3::{Digest, Sha3_256};

use chrono::Utc;



#[derive(Debug, Clone, Serialize, Deserialize)]
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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: [u8; 32],
    pub sender: String,
    pub recipient: String,
    pub signature: String,
    pub amount: f64,
    pub timestamp: usize,
    pub fee: usize,
    pub txn_type: TransactionType,
}



#[allow(dead_code)]
impl Transaction {
    pub fn new(
        sender: String, 
        recipient: String, 
        signature: String,
        amount: f64, 
        _timestamp: usize,
        fee: usize, 
        txn_type: TransactionType) -> Result<Self, String> {
        // TODO - hash should be generated based on all fields -> Placeholder for now
        // let hash = hash(&sender, &recipient, &amount, &fee, &txn_type);    
        Ok(Self {
                // TODO - hash should be generated based on all fields
                hash: [0; 32],
                sender,
                recipient,
                signature,
                amount,
                timestamp: Utc::now().timestamp_millis() as usize,
                fee,
                txn_type})
    }

    pub fn verify(&self) -> Result<bool, String> {
        let msg = serde_json::to_string(&self).unwrap();
        let public_key = PublicKey::from_bytes(&hex::decode(&self.sender).unwrap());
        Ok(public_key.verify(&msg.as_bytes(), &self.signature.as_bytes()))
    }
}
