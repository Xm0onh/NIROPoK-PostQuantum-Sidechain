use crate::block::Block;
use crate::mempool::Mempool;
use crate::accounts::{Account, State};
use crate::wallet::Wallet;
use crate::validator::Validator;
use crate::transaction::{Transaction, TransactionType};
use crate::config::STAKING_AMOUNT;
use crate::utils::Seed;
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub mempool: Mempool,
    pub wallet: Wallet,
    pub state: State,
    pub validator: Validator,
}


impl Blockchain {
    pub fn new(wallet: Wallet) -> Self {
        Self {
            chain: vec![],
            mempool: Mempool::new(),
            wallet,
            state: State::new(),
            validator: Validator::new(),
        }
    }

    pub fn select_block_proposer(&self) -> String {
        let mut lowest_weight = f64::INFINITY;
        String::new()
    }

    fn handle_transaction(&mut self, transaction: Transaction) {
        if transaction.txn_type == TransactionType::TRANSACTION {
            self.execute_transaction(transaction);
        }
        else if transaction.txn_type == TransactionType::STAKE {
            self.handle_stake(transaction);
        }
    }

    fn execute_transaction(&mut self, transaction: Transaction) {
        if transaction.verify().unwrap() {
            self.state.transfer(transaction.sender.clone(), transaction.recipient.clone(), transaction.amount);
        }
    }

    fn handle_stake(&mut self, transaction: Transaction) {
        if transaction.verify().unwrap() {
            self.validator.add_validator(transaction.sender.clone(), transaction.clone()).unwrap();
        }
    }
    // TODO
    // fn handle_unstake(&mut self, transaction: Transaction) {}
    
    pub fn verify_block(&mut self, block: Block) -> Result<bool, String> {
        let previous_block = self.chain.last().unwrap();
        if block.previous_hash != previous_block.hash {
            return Err("Previous block hash does not match".to_string());
        }
        // TODO - Verify the proposer
        Ok(true)
    }
    
    pub fn execute_block(&mut self, block: Block) {
        for txn in block.txn.clone() {
            if txn.verify().unwrap() {
                self.handle_transaction(txn);
            }
        }
        self.chain.push(block.clone());
        for txn in block.txn {
            self.mempool.delete_transaction(txn);
        }
    }

    pub fn get_validator(&self) -> &Validator {
        &self.validator
    }

    pub fn block_exists(&self, block: Block) -> bool {
        self.chain.iter().any(|b| b.id == block.id)
    }

}
