use super::GENESIS;
use super::hash::{H256};
use super::fork::{ForkBuffer};
use super::block::{Header};
use std::collections::{HashMap};
use super::primitive::block::{ContractState};
use super::experiment::snapshot::PERFORMANCE_COUNTER;

pub struct BlockChain {
    blockchain: Vec<ContractState>,
}

impl BlockChain {
    pub fn new() -> BlockChain {
        let genesis = ContractState::default();
        //PERFORMANCE_COUNTER.record_chain_update();
        BlockChain {
            blockchain: vec![genesis],
        } 
    }

    // input must be consistent with previous block
    pub fn insert(&mut self, contract_state: &ContractState) {
        self.blockchain.push(contract_state.clone());
    }

    // TODO redundent to insert, remove insert later
    pub fn append(&mut self, eth_state: &ContractState) {
        self.blockchain.push(eth_state.clone());
    }

    pub fn update(&mut self, eth_state: &ContractState) {
        let curr_state = self.blockchain.last();
        let curr_state = curr_state.expect("blockchain:update is empty");
        if eth_state.block_id == curr_state.block_id + 1 {
            self.blockchain.push(eth_state.clone());
        } else if eth_state.block_id > curr_state.block_id + 1 {
            // local chain is missing blocks
        } else if eth_state.block_id == curr_state.block_id {
            println!("local chain already synced");
        } else {
            panic!("local chain screw up, it is greater than eth chain");
        }
    }

    // block_id itself is changed
    pub fn revise(&mut self, block_id: usize, states: Vec<ContractState>) {

    }

    pub fn replace(&mut self, chain: Vec<ContractState>) {
        self.blockchain = chain;
        //PERFORMANCE_COUNTER.store_chain_depth(self.blockchain.len());
    }

    // block id should start at 0, so is consistent with height
    pub fn get_height(&self) -> u64 {
        self.blockchain.len() as u64
    }

    pub fn get_latest_state(&self) -> Option<ContractState> {
        match self.blockchain.last() {
            Some(c) => Some(c.clone()),
            None => None,
        }
    }
}
