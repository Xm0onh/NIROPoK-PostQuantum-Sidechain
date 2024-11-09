use crate::block::Block;
use crate::mempool::Mempool;
use crate::account::Account;
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

    fn verify_block(&mut self, block: Block) -> Result<bool, String> {
        let previous_block = self.chain.last().unwrap();
        if block.previous_hash != previous_block.hash {
            return Err("Previous block hash does not match".to_string());
        }
        Ok(true)
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
