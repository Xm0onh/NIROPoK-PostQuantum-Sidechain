use crate::block::Block;
use crate::mempool::Mempool;
use crate::account::Account;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use crate::wallet::Wallet;
use crate::validator::Validator;
use crate::transaction::Transaction;
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub mempool: Mempool,
    pub wallet: Wallet,
    pub accounts: Vec<Account>,
    pub validator: Validator,
}

impl Blockchain {
    pub fn new(wallet: Wallet) -> Self {
        Self {
            chain: vec![],
            mempool: Mempool::new(),
            wallet,
            accounts: vec![],
            validator: Validator::new(),
        }
    }

    fn execute_transaction(&mut self, transaction: Transaction) {
        if transaction.verify().unwrap() {
            self.accounts
                .iter_mut()
                .find(|a| a.accounts.contains(&transaction.sender))
                .unwrap()
                .balances.entry(transaction.sender.clone())
                .and_modify(|v| *v += transaction.amount)
                .or_insert(transaction.amount);


            self.accounts
                .iter_mut()
                .find(|a| a.accounts.contains(&transaction.recipient))
                .unwrap()
                .balances.entry(transaction.recipient.clone())
                .and_modify(|v| *v += transaction.amount)
                .or_insert(transaction.amount);
        }
    }

    fn execute_block(&mut self, block: Block) {
    }



    fn add_validator(&mut self, validator: Validator) {
        self.validator = validator;
    }

    pub fn get_validator(&self) -> &Validator {
        &self.validator
    }

}
