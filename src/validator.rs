
use std::collections::HashMap;
use crate::account::Account;
use crate::transaction::Transaction;
use crate::hashchain::HashChainMessage;
pub struct Validator {
    pub accounts: Vec<Account>,
    pub hash_chain_com: HashMap<String, HashChainMessage>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            accounts: vec![],
            hash_chain_com: HashMap::new(),
        }
    }

    pub fn add_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
        self.accounts.push(account);        
        Ok(true)
    }

    pub fn remove_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
        self.accounts.retain(|a| a != &account);
        Ok(true)
    }

    pub fn update_validator_com(&mut self, address: String, com: HashChainMessage) {
        self.hash_chain_com.insert(address, com);
    }
}
