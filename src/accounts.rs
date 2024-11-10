use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub struct Account {
    pub address: String
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct State {
    pub accounts: Vec<Account>,
    pub balances: HashMap<Account, f64>,
}

impl State {
    pub fn new() -> Self {
        State { accounts: Vec::new(), balances: HashMap::new() }
    }

    pub fn add_account(&mut self, account: Account) {
        if !self.balances.contains_key(&account) {
            self.balances.insert(account.clone(), 0.00);
            self.accounts.push(account);
        }
    }

    pub fn remove_account(&mut self, account: Account) {
        self.balances.remove(&account);
        self.accounts.retain(|a| a != &account);
    }

    pub fn transfer(&mut self, from: Account, to: Account, amount: f64) {
        self.balances.entry(from).and_modify(|v| *v -= amount);
        self.balances.entry(to).and_modify(|v| *v += amount);
    }

    pub fn stake(&mut self, account: Account, amount: f64) {
        self.balances.entry(account).and_modify(|v| *v += amount);
    }

    pub fn unstake(&mut self, account: Account, amount: f64) {
        self.balances.entry(account).and_modify(|v| *v -= amount);
    }

    pub fn get_balance(&self, account: Account) -> f64 {
        *self.balances.get(&account).unwrap_or(&0.00)
    }
}
