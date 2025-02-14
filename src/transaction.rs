use crate::accounts::Account;
use crate::wallet::Wallet;
use chrono::Utc;
use crystals_dilithium::dilithium2::{PublicKey, Signature};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha3::{Digest, Sha3_256};

// Custom serialization for Signature
fn serialize_signature<S>(signature: &Signature, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Serialize the signature bytes directly
    serializer.serialize_bytes(signature.as_ref())
}

// Custom deserialization for Signature
fn deserialize_signature<'de, D>(deserializer: D) -> Result<Signature, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
    Signature::try_from(bytes.as_slice()).map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    TRANSACTION,
    COINBASE,
    STAKE,
    UNSTAKE,
    DELEGATE,
    RESIGN,
    VALIDATOR,
    ValidatorReward,
    COMMIT,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: [u8; 32],
    pub sender: Account,
    pub recipient: Account,
    #[serde(serialize_with = "serialize_signature")]
    #[serde(deserialize_with = "deserialize_signature")]
    pub signature: Signature,
    pub amount: f64,
    pub timestamp: usize,
    pub fee: usize,
    pub txn_type: TransactionType,
}

impl Transaction {
    pub fn new(
        sender_wallet: &mut Wallet,
        sender: Account,
        recipient: Account,
        amount: f64,
        fee: usize,
        txn_type: TransactionType,
    ) -> Result<Self, String> {
        let timestamp = Utc::now().timestamp_millis() as usize;
        let mut txn = Self {
            hash: [0u8; 32],
            sender,
            recipient,
            signature: sender_wallet.sign_message(&[0u8; 32]),
            amount,
            timestamp,
            fee,
            txn_type,
        };
        txn.hash = txn.compute_hash();
        txn.signature = sender_wallet.sign_message(&txn.hash);
        Ok(txn)
    }

    pub fn verify(&self) -> Result<bool, String> {
        let msg = &self.hash;
        let public_key = PublicKey::from_bytes(&hex::decode(&self.sender.address).unwrap());
        Ok(public_key.verify(msg, &self.signature))
    }

    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(self.sender.address.as_bytes());
        hasher.update(self.recipient.address.as_bytes());
        hasher.update(self.amount.to_string().as_bytes());
        hasher.update(self.timestamp.to_string().as_bytes());
        hasher.update(self.fee.to_string().as_bytes());
        hasher.update(serde_json::to_string(&self.txn_type).unwrap().as_bytes());
        hasher.finalize().into()
    }
}
