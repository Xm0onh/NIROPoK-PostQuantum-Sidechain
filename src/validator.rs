use crate::accounts::{Account, State};
use crate::config::STAKING_AMOUNT;
use crate::hashchain::HashChainCom;
use crate::transaction::{Transaction, TransactionType};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Validator {
    pub state: State,
    pub hash_chain_com: HashMap<String, HashChainCom>,
    pub next_block_hash: HashMap<Account, String>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            state: State::new(),
            hash_chain_com: HashMap::new(),
            next_block_hash: HashMap::new(),
        }
    }

    pub fn add_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
        self.state.add_account(account.clone());
        if txn.txn_type == TransactionType::STAKE && txn.amount >= STAKING_AMOUNT {
            self.state.stake(account.clone(), txn.amount);
            self.state.balances.insert(account.clone(), txn.amount);
            // self.state.accounts.push(account.clone());
        }
        Ok(true)
    }

    pub fn apply_buffer(&mut self, accounts: Vec<Account>, txns: Vec<Transaction>) {
        // update the list of validators by calling add_validator for each account
        for (i, account) in accounts.iter().enumerate() {
            self.add_validator(account.clone(), txns.get(i).unwrap().clone())
                .unwrap();
        }
    }

    // pub fn remove_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
    //     self.state.remove_account(account);
    //     Ok(true)
    // }

    pub fn get_validator_commitment(&self, account: Account) -> &HashChainCom {
        if let Some(hash_chain_message) = self.hash_chain_com.get(&account.address) {
            hash_chain_message
        } else {
            panic!(
                "Hash chain message not found for account: {:?}",
                account.address
            );
        }
    }

    pub fn update_validator_com(&mut self, account: Account, com: HashChainCom) {
        self.hash_chain_com.insert(account.address, com);
    }

    #[allow(dead_code)]
    pub fn reset_validator_com(&mut self) {
        self.hash_chain_com.clear();
    }

    #[allow(dead_code)]
    pub fn hash_chain_received(&self) -> bool {
        self.hash_chain_com.len() == self.state.accounts.len()
    }
}
