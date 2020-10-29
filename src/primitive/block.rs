use serde::{Serialize, Deserialize};
use super::hash::{H256};
use super::crypto::{self};
use super::merkle::{MerkleHash};

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ContractState {
    pub curr_hash: H256,
    pub block_id: u64,
}

impl ContractState {
    pub fn genesis () -> ContractState {
        ContractState {
            curr_hash: H256::zero(),
            block_id: 0,
        }
    }
}



#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default, PartialEq, Eq)]
pub struct EthBlkTransaction {
     pub contract_state: ContractState,
     pub block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default, PartialEq, Eq)]
pub struct Block {
    pub header: Header,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn insert(&mut self, transaction: Transaction) {
        self.transactions.push(transaction); 
    }

    pub fn clear(&mut self) {
        self.transactions.clear();
    }

    pub fn len(&self) -> usize {
        self.transactions.len() 
    }

    pub fn update_nonce(&mut self, nonce: H256) -> H256 {
        self.header.nonce = nonce;
        self.update_hash()
    }

    pub fn update_hash(&mut self) -> H256 {
        let ser = bincode::serialize(self).unwrap();
        let hash = crypto::hash(&ser);
        self.header.hash = hash;
        self.header.hash
    }

    pub fn update_root(&mut self) -> H256 {
        let merkle_root = MerkleHash::new(&self.transactions);

        self.header.root = merkle_root;
        self.header.root
    }

    pub fn update_prev_blockhash(&mut self, hash: H256) {
        self.header.prev_hash = hash;
    }

    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}



#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default, PartialEq, Eq)]
pub struct Header {
    pub hash: H256,
    pub nonce: H256,
    pub height: usize,
    pub root: H256,
    pub prev_hash: H256,
}


#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default, PartialEq, Eq)]
pub struct Transaction {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub is_coinbase: bool,
    #[serde(skip)]
    pub hash: H256
}

impl Transaction {
    pub fn update_hash(&mut self) {
        let ser = bincode::serialize(&self).expect("unable to encode msg");
        self.hash = crypto::hash(ser.as_slice());
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default, PartialEq, Eq)]
pub struct Input {
    pub tx_hash: H256,
    pub index: u8,
    pub unlock: H256,
    pub content: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Default, PartialEq, Eq)]
pub struct Output {
    pub address: H256, //lock
    pub value: u64,
}

