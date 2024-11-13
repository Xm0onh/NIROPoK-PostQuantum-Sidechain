use crate::block::Block;
use crate::mempool::Mempool;
use crate::accounts::{Account, State};
use crate::wallet::Wallet;
use crate::validator::Validator;
use crate::transaction::{Transaction, TransactionType};
use crate::utils::{Seed, select_block_proposer, get_block_seed};
use crate::epoch::Epoch;
use crate::hashchain::{HashChain, verify_hash_chain_index};
use log::info;
#[allow(unused_imports)]
use crate::config::EPOCH_DURATION;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub mempool: Mempool,
    pub wallet: Wallet,
    pub state: State,
    pub validator: Validator,
    pub epoch: Epoch,
    pub buffer: Buffer,
    pub hash_chain: HashChain,
}

pub struct Buffer {
    pub accounts: Vec<Account>,
    pub txns: Vec<Transaction>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            accounts: vec![],
            txns: vec![],
        }
    }

    pub fn reset(&mut self) {
        self.accounts.clear();
        self.txns.clear();
    }
}

impl Blockchain {
    pub fn new(wallet: Wallet) -> Self {
        let mut blockchain = Self {
            chain: vec![],
            mempool: Mempool::new(),
            wallet,
            state: State::new(),
            validator: Validator::new(),
            epoch: Epoch::new(),
            buffer: Buffer::new(),
            hash_chain: HashChain { hash_chain: vec![] },
        };
        let wallet = &mut blockchain.wallet;
        // fun the account as the first validator
        let account = Account { address: wallet.get_public_key().to_string() };

        blockchain.validator.add_validator(account.clone(), Transaction::new(
            wallet,
            account.clone(),
            account.clone(),
            100.00,
            0,
            TransactionType::STAKE
        ).unwrap()).unwrap();
        blockchain.fund_wallet(10000.00);
        blockchain
    }

    pub fn select_block_proposer(&self, seed: Seed) -> &Account {
        select_block_proposer(seed, &self.validator)
    }

    pub fn new_epoch(&mut self) -> Seed {
        Seed::new_epoch_seed(&self.validator)
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
            // Add to buffer
            self.buffer.accounts.push(transaction.sender.clone());
            self.buffer.txns.push(transaction.clone());
        }
    }

    pub fn end_of_epoch(&mut self) {
        self.validator.apply_buffer(self.buffer.accounts.clone(), self.buffer.txns.clone());
        self.buffer.reset();
        self.epoch.reset();
    }
    // TODO
    // fn handle_unstake(&mut self, transaction: Transaction) {}
    
    pub fn get_next_seed(&self) -> Seed {
        let latest_block = self.chain.last();
        if let Some(block) = latest_block {
            get_block_seed(block.proposer_hash.clone(), block.seed.get_seed())
        } else {
            // print the error
            info!("No blocks in the chain");
            Seed::new_epoch_seed(&self.validator)
        }
    }

    pub fn propose_block(&mut self, proposer_hash: String, proposer_address: Account, txns: Vec<Transaction>, seed: Seed ) -> Block {
        // If the chain is empty, we need to create the first block
        if self.chain.is_empty() {
            let block = Block::new(
                0, 
                [0; 32], 
                self.epoch.timestamp as usize, 
                vec![], 
                proposer_address, 
                proposer_hash, 
                seed
            ).unwrap();
            block
        } else {
            let latest_block = self.chain.last().unwrap();
            let block = Block::new(
                latest_block.id + 1, 
                latest_block.hash, 
                latest_block.timestamp, 
                txns, 
                proposer_address, 
                proposer_hash, 
                seed
            ).unwrap();
            block
        }
    }


    pub fn verify_block(&mut self, block: Block) -> Result<bool, String> {
        // check if this is the genesis block
        if block.id == 0 {
            return Ok(true);
        }
        let previous_block = self.chain.last().unwrap();
        if block.previous_hash != previous_block.hash {
            return Err("Previous block hash does not match".to_string());
        }

        let proposer_address = block.proposer_address;
        let proposer_commtiment = self.validator.get_validator_commitment(proposer_address);
        if !verify_hash_chain_index(block.proposer_hash, self.epoch.timestamp as u64, proposer_commtiment) {
            return Err("Hash chain index does not match".to_string());
        }
        Ok(true)
    }
    
    pub fn execute_block(&mut self, block: Block) {
        // if txns, do nothing
        if block.txn.is_empty() {
            info!("Block has no transactions");
            self.chain.push(block.clone());
            return;
        }
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

    pub fn get_validators(&self) -> &Validator {
        &self.validator
    }

    pub fn block_exists(&self, block: Block) -> bool {
        self.chain.iter().any(|b| b.id == block.id)
    }


    // Just a function for testing - funding the wallet
    pub fn fund_wallet(&mut self, amount: f64) {
        self.state.balances.insert(Account { address: self.wallet.get_public_key().to_string() }, amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_blockchain() -> Blockchain {
        let wallet = Wallet::new().unwrap();
        Blockchain::new(wallet)
    }

    #[test]
    fn test_select_block_proposer() {
        let mut blockchain = setup_blockchain();
        
        let mut wallet1 = Wallet::new().unwrap();
        let validator1 = Account { address: wallet1.get_public_key()};
        let mut wallet2 = Wallet::new().unwrap();
        let validator2 = Account { address: wallet2.get_public_key() };
        


        let stake_txn1 = Transaction::new(
            &mut wallet1,
            validator1.clone(),
            validator1.clone(),
            100.0,
            0,
            TransactionType::STAKE,
            ).unwrap();
        let stake_txn2 = Transaction::new(
            &mut wallet2,
            validator2.clone(),
            validator2.clone(),
            200.0,
            0,
            TransactionType::STAKE,
            ).unwrap();

        blockchain.handle_stake(stake_txn1);
        blockchain.handle_stake(stake_txn2);
        blockchain.end_of_epoch();
        // Hash chain
        let hash_chain_validator1 = HashChain::new();
        let hash_chain_validator2 = HashChain::new();

        blockchain.validator.update_validator_com(validator1.clone(), hash_chain_validator1.get_hash(EPOCH_DURATION as usize));
        blockchain.validator.update_validator_com(validator2.clone(), hash_chain_validator2.get_hash(EPOCH_DURATION as usize));

        let seed = blockchain.new_epoch();
        let proposer = blockchain.select_block_proposer(seed);
        println!("Proposer: {}", proposer.address);
        assert!(
            proposer.address == validator1.address || 
            proposer.address == validator2.address
        );
    }
}

