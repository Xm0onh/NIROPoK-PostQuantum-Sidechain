use std::collections::HashMap;

pub struct Stake {
    pub accounts: Vec<String>,
    pub balances: HashMap<String, f64>,
}

impl Stake {
    pub fn new() -> Self {
        Self {
            accounts: vec![],
            balances: HashMap::new(),
        }
    }

    pub fn stake(&mut self, account: &String, amount: f64) {
        if !self.balances.contains_key(account) {
            self.balances.insert(account.to_string(), 0.0);
        }
        self.balances.entry(account.to_string()).and_modify(|v| *v += amount);
    }

    pub fn unstake(&mut self, account: String, amount: f64) {
        if let Some(balance) = self.balances.get_mut(&account) {
            *balance -= amount;     
            if *balance <= 0.0 {
                self.balances.remove(&account);
            }
        }
    }

    pub fn get_balance(&self, account: String) -> f64 {
        *self.balances.get(&account).unwrap_or(&0.0)
    }

    pub fn get_all_balances(&self) -> HashMap<String, f64> {
        self.balances.clone()
    }
    

}
