
use std::collections::HashMap;
use crate::accounts::{Account, State};
use crate::transaction::{Transaction, TransactionType};
use crate::hashchain::HashChainMessage;
use crate::config::STAKING_AMOUNT;
pub struct Validator {
    pub state: State,
    pub hash_chain_com: HashMap<String, HashChainMessage>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            state: State::new(),
            hash_chain_com: HashMap::new(),
        }
    }

    pub fn add_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
        self.state.add_account(account.clone());
        if txn.txn_type == TransactionType::STAKE && txn.amount >= STAKING_AMOUNT {
            self.state.stake(account, txn.amount);
        }
        Ok(true)
    }

    pub fn remove_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
        self.state.remove_account(account);
        Ok(true)
    }

    pub fn update_validator_com(&mut self, address: String, com: HashChainMessage) {
        self.hash_chain_com.insert(address, com);
    }

    pub fn reset_validator_com(&mut self) {
        self.hash_chain_com.clear();
    }
}
