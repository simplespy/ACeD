use super::hash::{H256};
use super::block::{Block};
use std::sync::{Mutex, Arc};
use std::collections::{HashMap};

pub struct Location {
    block_height: usize,
    index: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct Utxo {
    pub hash: H256,
    pub index: u8,
}

pub struct UtxoDb {
    pub utxo_db: HashMap<H256, Location>,
}

impl UtxoDb {
    pub fn new() -> UtxoDb {
        UtxoDb {
            utxo_db: HashMap::new(), 
        } 
    }
}
