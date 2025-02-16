use crate::accounts::{Account, State};
use crate::block::Block;
use crate::ccok::{Builder as CertBuilder, Certificate, Params, Participant};
#[allow(unused_imports)]
use crate::config::EPOCH_DURATION;
use crate::epoch::Epoch;
use crate::hashchain::{verify_hash_chain_index, HashChain};
use crate::mempool::Mempool;
use crate::merkle::MerkleTreeBuilder;
use crate::p2p::BlockSignature;
use crate::transaction::{Transaction, TransactionType};
use crate::utils::{get_block_seed, select_block_proposer, Seed};
use crate::validator::Validator;
use crate::wallet::Wallet;
use hex;
use log::{error, info, warn};
use std::collections::HashMap;
use std::convert::TryInto;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub mempool: Mempool,
    pub wallet: Wallet,
    pub state: State,
    pub validator: Validator,
    pub epoch: Epoch,
    pub buffer: Buffer,
    pub hash_chain: HashChain,
    pub pending_signatures: HashMap<usize, Vec<BlockSignature>>,
    pub last_certificate: Option<(usize, Certificate)>,
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
            pending_signatures: HashMap::new(),
            last_certificate: None,
        };
        let wallet = &mut blockchain.wallet;
        let account = Account {
            address: wallet.get_public_key().to_string(),
        };
        blockchain
            .validator
            .add_validator(
                account.clone(),
                Transaction::new(
                    wallet,
                    account.clone(),
                    account.clone(),
                    100.00,
                    0,
                    TransactionType::STAKE,
                )
                .unwrap(),
            )
            .unwrap();
        warn!(
            "Size of validator: {:?}",
            blockchain.validator.state.accounts.len()
        );
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
        } else if transaction.txn_type == TransactionType::STAKE {
            self.handle_stake(transaction);
        }
    }

    fn execute_transaction(&mut self, transaction: Transaction) {
        if transaction.verify().unwrap() {
            self.state.transfer(
                transaction.sender.clone(),
                transaction.recipient.clone(),
                transaction.amount,
            );
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
        self.validator
            .apply_buffer(self.buffer.accounts.clone(), self.buffer.txns.clone());
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
            error!("No blocks in the chain");
            Seed::new_epoch_seed(&self.validator)
        }
    }

    pub fn propose_block(
        &mut self,
        proposer_hash: String,
        proposer_address: Account,
        txns: Vec<Transaction>,
        seed: Seed,
    ) -> Block {
        // Check if the last certificate corresponds to the immediate previous block.
        let cert_to_attach = if let Some((cert_block_id, cert)) = self.last_certificate.take() {
            let last_block_id = self.chain.last().map(|b| b.id).unwrap_or(0);
            if cert_block_id == last_block_id {
                info!("Proposing block with certificate computed for previous block: {}", cert_block_id);
                Some(cert)
            } else {
                info!(
                    "Ignoring certificate computed for block {} (previous block is {})",
                    cert_block_id, last_block_id
                );
                None
            }
        } else {
            info!("Proposing block without certificate");
            None
        };

        self.propose_block_with_certificate(proposer_hash, proposer_address, txns, seed, cert_to_attach)
    }

    pub fn propose_block_with_certificate(
        &mut self,
        proposer_hash: String,
        proposer_address: Account,
        txns: Vec<Transaction>,
        seed: Seed,
        certificate: Option<Certificate>,
    ) -> Block {
        let block = if self.chain.is_empty() {
            Block::new(
                1,
                [0; 32],
                self.epoch.timestamp as usize,
                vec![],
                proposer_address,
                proposer_hash,
                seed,
                certificate,
            )
            .unwrap()
        } else {
            let latest_block = self.chain.last().unwrap();
            Block::new(
                latest_block.id + 1,
                latest_block.hash,
                latest_block.timestamp,
                txns,
                proposer_address,
                proposer_hash,
                seed,
                certificate,
            )
            .unwrap()
        };

        let block_hash_str = hex::encode(&block.hash);
        let local_pub = self.wallet.get_public_key().to_string();
        let signature = self.wallet.sign_message(block_hash_str.as_bytes());
        let block_sig = crate::p2p::BlockSignature {
            block_id: block.id,
            block_hash: block_hash_str, // The signed message is now the block hash.
            sender: crate::accounts::Account { address: local_pub },
            signature: signature.to_vec(),
        };
        self.collect_block_signature(block_sig);

        block
    }

    pub fn verify_block(&mut self, block: Block) -> bool {
        if block.id == 1 {
            return true;
        }
        let previous_block = self.chain.last().unwrap();
        if block.previous_hash != previous_block.hash {
            error!("Previous block hash does not match");
        } else {
            info!("✅ Previous block hash matches");
        }

        let proposer_address = block.proposer_address;
        let proposer_commtiment = self.validator.get_validator_commitment(proposer_address);
        if !verify_hash_chain_index(
            proposer_commtiment.hash_chain_index.clone(),
            self.epoch.timestamp,
            block.proposer_hash.clone(),
        ) {
            error!("Hash chain index does not match");
            return false;
        }
        true
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

    #[allow(dead_code)]
    pub fn get_validators(&self) -> &Validator {
        &self.validator
    }

    pub fn block_exists(&self, block: Block) -> bool {
        self.chain.iter().any(|b| b.id == block.id)
    }

    pub fn get_latest_block_id(&self) -> u64 {
        if self.chain.is_empty() {
            1
        } else {
            self.chain.last().unwrap().id as u64
        }
    }

    // Just a function for testing - funding the wallet
    pub fn fund_wallet(&mut self, amount: f64) {
        self.state.balances.insert(
            Account {
                address: self.wallet.get_public_key().to_string(),
            },
            amount,
        );
    }

    pub fn collect_block_signature(&mut self, block_sig: BlockSignature) {
        let expected = self.validator.state.accounts.len();
        let (should_build, block_id, block_hash) = {
            let sigs = self
                .pending_signatures
                .entry(block_sig.block_id)
                .or_insert(vec![]);
            let sender_address = block_sig.sender.address.clone();
            if !sigs.iter().any(|s| s.sender.address == sender_address) {
                sigs.push(block_sig);
            }
            let should_build = sigs.len() >= expected;
            let block_id = sigs[0].block_id;
            let block_hash = sigs[0].block_hash.clone();
            (should_build, block_id, block_hash)
        };

        if should_build {
            let mut params = Params {
                msg: block_hash.as_bytes().to_vec(),
                proven_weight: 0,
                security_param: 128,
            };
            // Compute proven_weight while building participants.
            let participants: Vec<Participant> = self
                .validator
                .state
                .accounts
                .iter()
                .map(|a| {
                    let weight =
                        self.validator.state.balances.get(a).cloned().unwrap_or(0.0) as u64;
                    params.proven_weight += weight;
                    Participant {
                        public_key: a.address.clone(),
                        weight,
                    }
                })
                .collect();

            let collected_sigs = self
                .pending_signatures
                .remove(&block_id)
                .unwrap_or_else(Vec::new);
            // Build the party tree from participants as in the test.
            let mut tree = MerkleTreeBuilder::new();
            tree.build(&participants)
                .expect("Failed to build Merkle tree");
            let party_tree_root = tree.root();
            let mut builder = CertBuilder::new(params, participants.clone(), party_tree_root);
            // For each collected block signature, add the signature to the builder.
            for sig in collected_sigs {
                if let Some(idx) = participants
                    .iter()
                    .position(|p| p.public_key == sig.sender.address)
                {
                    let fixed_sig: [u8; 2420] = sig
                        .signature
                        .try_into()
                        .expect("Signature length does not match expected size");
                    let _ = builder.add_signature(idx, fixed_sig);
                }
            }
            let certificate = match builder.build() {
                Ok(cert) => cert,
                Err(e) => {
                    error!("Error building certificate: {}", e);
                    return;
                }
            };

            info!("🔐 Certificate computed for block {}: {:?}", block_id, certificate.proof_size());
            self.last_certificate = Some((block_id, certificate));
        }
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
        let validator1 = Account {
            address: wallet1.get_public_key(),
        };
        let mut wallet2 = Wallet::new().unwrap();
        let validator2 = Account {
            address: wallet2.get_public_key(),
        };

        let stake_txn1 = Transaction::new(
            &mut wallet1,
            validator1.clone(),
            validator1.clone(),
            100.0,
            0,
            TransactionType::STAKE,
        )
        .unwrap();
        let stake_txn2 = Transaction::new(
            &mut wallet2,
            validator2.clone(),
            validator2.clone(),
            200.0,
            0,
            TransactionType::STAKE,
        )
        .unwrap();

        blockchain.handle_stake(stake_txn1);
        blockchain.handle_stake(stake_txn2);
        blockchain.end_of_epoch();
        // Hash chain
        let hash_chain_validator1 = HashChain::new();
        let hash_chain_validator2 = HashChain::new();

        let val1_account = Account {
            address: validator1.address.clone(),
        };
        let val2_account = Account {
            address: validator2.address.clone(),
        };

        blockchain.validator.update_validator_com(
            val1_account.clone(),
            hash_chain_validator1.get_hash(EPOCH_DURATION as usize, val1_account.clone()),
        );
        blockchain.validator.update_validator_com(
            val2_account.clone(),
            hash_chain_validator2.get_hash(EPOCH_DURATION as usize, val2_account.clone()),
        );

        let seed = blockchain.new_epoch();
        let proposer = blockchain.select_block_proposer(seed);
        if proposer.address == validator1.address {
            println!("Validator 1 is proposer");
        } else if proposer.address == validator2.address {
            println!("Validator 2 is proposer");
        }
        assert!(proposer.address == validator1.address || proposer.address == validator2.address);
    }
}
