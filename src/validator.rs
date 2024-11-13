
use std::collections::HashMap;
use crate::accounts::{Account, State};
use crate::transaction::{Transaction, TransactionType};
use crate::hashchain::HashChainMessage;
use crate::config::STAKING_AMOUNT;

#[derive(Debug)]
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

    pub fn apply_buffer(&mut self, accounts: Vec<Account>, txns: Vec<Transaction>) {
        // update the list of validators by calling add_validator for each account
        for (i, account) in accounts.iter().enumerate() {
            self.add_validator(account.clone(), txns.get(i).unwrap().clone()).unwrap();
        }
    }

    // pub fn remove_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
    //     self.state.remove_account(account);
    //     Ok(true)
    // }

    pub fn get_validator_commitment(&self, account: Account) -> &HashChainMessage {
        self.hash_chain_com.get(&account.address).unwrap()
    }

    pub fn update_validator_com(&mut self, account: Account, com: HashChainMessage) {
        self.hash_chain_com.insert(account.address, com);
    }

    #[allow(dead_code)]
    pub fn reset_validator_com(&mut self) {
        self.hash_chain_com.clear();
    }

    pub fn hash_chain_received(&self) -> bool {
        self.hash_chain_com.len() == self.state.accounts.len()
    }
}
