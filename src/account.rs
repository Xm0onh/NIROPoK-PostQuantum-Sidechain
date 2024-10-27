use serde::{Serialize, Deserialize};
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Account {
    pub accounts: Vec<String>,
    pub balances: HashMap<String, f64>,
}

impl Account {
    pub fn new() -> Self {
        Account { accounts: Vec::new(), balances: HashMap::new() }
    }

    pub fn add_account(&mut self, address: &String) {
        if !self.balances.contains_key(address) {
            self.balances.insert(address.to_string(), 0.00);
            self.accounts.push(address.to_string());
        }
    }

    pub fn transfer(&mut self, from: &String, to: &String, amount: f64) {
        self.balances.entry(from.to_string()).and_modify(|v| *v -= amount);
        self.balances.entry(to.to_string()).and_modify(|v| *v += amount);
    }

    pub fn get_balance(&self, address: &String) -> f64 {
        *self.balances.get(address).unwrap_or(&0.00)
    }
    
}
