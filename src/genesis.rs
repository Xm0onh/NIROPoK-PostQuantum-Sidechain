use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Genesis {
    pub stake_txn: Transaction,
}

impl Genesis {
    pub fn new(stake_txn: Transaction) -> Self {
        Self { stake_txn }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;
    use crate::accounts::Account;
    use crate::transaction::TransactionType;

    #[test]
    fn test_genesis_serialization() {
        // Create a new wallet for testing
        let mut wallet = Wallet::new().unwrap();
        let account = Account { address: wallet.get_public_key().to_string() };
        
        // Create a stake transaction
        let stake_txn = Transaction::new(
            &mut wallet,
            account.clone(),
            account.clone(),
            1000.0,
            0,
            TransactionType::STAKE
        ).unwrap();

        // Create a hash chain message
        // Create Genesis instance
        let genesis = Genesis::new(stake_txn.clone());

        // Serialize to bytes instead of JSON string
        let serialized = serde_json::to_vec(&genesis).unwrap();

        // Deserialize using from_slice instead of from_str
        let deserialized: Genesis = serde_json::from_slice(&serialized).unwrap();

        // Verify the data matches
        assert_eq!(deserialized.stake_txn.hash, genesis.stake_txn.hash);
        assert_eq!(deserialized.stake_txn.amount, genesis.stake_txn.amount);
        assert_eq!(deserialized.stake_txn.sender.address, genesis.stake_txn.sender.address);
        assert_eq!(deserialized.stake_txn.recipient.address, genesis.stake_txn.recipient.address);
        assert_eq!(deserialized.stake_txn.txn_type, genesis.stake_txn.txn_type);
    }
}