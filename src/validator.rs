use crate::account::Account;
use crate::transaction::Transaction;

pub struct Validator {
    pub accounts: Vec<Account>,
}

pub const MIN_STAKE: f64 = 100.0;

impl Validator {
    pub fn new() -> Self {
        Self {
            accounts: vec![],
        }
    }

    pub fn add_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
        if *account.balances.get(&txn.sender).unwrap_or(&0.0) < MIN_STAKE {
            return Err("Account does not have enough balance to be a validator".to_string());
        } 
        self.accounts.push(account);        
        Ok(true)
    }

    pub fn remove_validator(&mut self, account: Account, txn: Transaction) -> Result<bool, String> {
        self.accounts.retain(|a| a != &account);
        Ok(true)
    }
}
