use std::collections::{HashMap};
use super::hash::{H256};
use super::block::{Block, Header};
use super::blockchain::{BlockChain};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self};
use std::collections::{HashSet, VecDeque};
use super::contract::interface::{Message, Handle, Answer};
use web3::types::{TransactionReceipt};
use super::contract::interface::Response as ContractResponse;
use crossbeam::channel::{self, Sender};
use super::scheduler::{Token};

use super::cmtda::Block as CMTBlock;
use super::cmtda::Transaction;
use super::cmtda::H256 as CMTH256;

use super::cmtda::{BlockHeader, HEADER_SIZE, read_codes};
use chain::decoder::{Code, Decoder, TreeDecoder, CodingErr, IncorrectCodingProof};
use chain::decoder::{Symbol};
use chain::constants::{BLOCK_SIZE, BASE_SYMBOL_SIZE, RATE};
use primitives::bytes::{Bytes};
use ser::{deserialize, serialize};
use std::net::{SocketAddr};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};
use merkle;
use ring::digest::{Algorithm, Context, SHA256};
use merkle::{Hashable, MerkleTree, Proof};
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::mempool::scheduler;
#[allow(non_upper_case_globals)]
static algorithm: &'static Algorithm = &SHA256;

//struct Sample {
    //symbols: Vec<Vec<Symbols>>,
    //indices: Vec<Vec<u64>>,
//}

//impl Sample {
    //fn merge(&mut self, s: &Sample) {
        //let num_layer = self.symbols.len();
        //for l in 0..num_layer {
            //let symbols_l = &s.symbols[l];
            //let idx_l= &s.idx[l];
            //let mut c_symbols = &mut self.symbols[l];
            //let mut c_idx = &mut self.indices[l];
            //let mut j = 0;
            //for i in idx {
                //if c_idx.contains(i) {
                    ////
                //} else {
                    //c_idx.push(*i);
                    //c_symbols.push(symbols[j]);
                //}
                //j += 1;
            //}
        //}
    //}
//}

pub struct Mempool {
    transactions: VecDeque<Transaction>,
    block_size: usize,
    contract_handler: Sender<Handle>,
    schedule_handler: Sender<scheduler::Signal>,
    returned_blocks: VecDeque<Block>,
    symbols_by: HashMap<u64, HashMap<u64, (Vec<Vec<Symbol>>, Vec<Vec<u64>>) > >,
    headers_by: HashMap<u64, BlockHeader>,
    //block_by: HashMap<u64, CMTBlock>,
    addr: SocketAddr,
    codes_for_encoding: Vec<Code>,
    codes_for_decoding: Vec<Code>,
}

impl Mempool {
    pub fn new(
        contract_handler: Sender<Handle>,
        schedule_handler: Sender<scheduler::Signal>,
        addr: SocketAddr,
        codes_for_encoding: Vec<Code>,
        codes_for_decoding: Vec<Code>,
    ) -> Mempool {
        
        Mempool {
            transactions: VecDeque::with_capacity(200000), 
            block_size: BLOCK_SIZE as usize, // in bytes
            contract_handler: contract_handler,
            schedule_handler: schedule_handler,
            returned_blocks: VecDeque::new(),
            symbols_by: HashMap::new(),
            //block_by: HashMap::new(),
            addr: addr,
            codes_for_encoding: codes_for_encoding,
            codes_for_decoding: codes_for_decoding,
            headers_by: HashMap::new(),
        } 
    }

    pub fn transaction_size_in_bytes(&self) -> usize {
        let mut trans_byte = self.transactions.iter().map(Transaction::bytes).collect::<Vec<Bytes>>();
        let mut total_size = 0;
        for tx in &trans_byte {
            total_size +=  tx.len();
        }
        total_size
    }

    pub fn change_mempool_size(&mut self, size: usize) {
        self.block_size = size;
    }

    pub fn get_num_transaction(&self) -> u64 {
        return self.transactions.len() as u64;
    }

    // TODO change the height in the header, but leave to future when tx has meaning
    pub fn return_block(&mut self, block: Block) {
        self.returned_blocks.push_back(block);
    }

    pub fn insert_symbols(
        &mut self, 
        block_id: u64, 
        block_header: &BlockHeader,
        symbols_by_scale_id: HashMap<u64, (Vec<Vec<Symbol>>, Vec<Vec<u64>>)>
    ) {
        self.headers_by.insert(block_id, block_header.clone());
        self.symbols_by.insert(block_id, symbols_by_scale_id);
    }

    pub fn get_cmt_sample(&mut self, block_id: u64, scale_id: u64) 
        -> (BlockHeader, Vec<Vec<Symbol>>, Vec<Vec<u64>>) {
         match self.symbols_by.get(&block_id) {
            Some(symbols_by) => {
                match symbols_by.get(&scale_id) {
                    Some((s, i)) => {
                        let header = match self.headers_by.get(&block_id) {
                            Some(h) => h.clone(),
                            None => {
                                info!("I don't have cmt header for block id {}", block_id);
                                unreachable!();
                            },
                        };
                        return (header, s.clone(), i.clone())
                    },
                    None => {
                        info!("I have cmt symbols for block id {}, but not have for scale node {}", block_id, scale_id);
                        unreachable!();
                    }
                }
            },
            None => {
                info!("I don't have cmt symbols for block id {}", block_id);
                unreachable!();
            }
         }
    }

    //pub fn sample_cmt(&mut self, 
        //block_id: u64, 
        //sample_idx: Vec<u32>,
    //)-> (BlockHeader, Vec<Vec<Symbol>>, Vec<Vec<u64>>) {
        //match self.block_by.get(&block_id) {
            //None => {
                //info!("I don't have cmt block {}", block_id);
                //unreachable!();
            //},
            //Some(cmt_block) => {
                //let num = sample_idx.len();
                //let (mut symbols, mut idx) = cmt_block.sample_vec(sample_idx);

                ////info!("{:?}, symbols {:?}", self.addr, symbols);
                ////info!("{:?}, idx     {:?}", self.addr,idx);

                ////let mut decoder: TreeDecoder = TreeDecoder::new(
                    ////self.codes_for_decoding.to_vec(), 
                    ////&cmt_block.block_header.coded_merkle_roots_hashes
                ////);

                ////info!("{:?}, test treedecoder n {} height {}", self.addr, decoder.n, decoder.height);
                ////match decoder.run_tree_decoder(symbols.clone(), idx.clone(), cmt_block.block_header.clone()) {
                    ////Ok(transactions) => {
                        ////println!("transactions {:?}", transactions);

                    ////},
                    ////_ => info!("tree decoder error"),
                ////};
                ////info!("{:?} after calling tree decoder", self.addr);
                //(cmt_block.block_header.clone(), symbols, idx)
            //}
        //}
    //}

    //pub fn update_block_id(&mut self, new_id: u64, old_id: u64) {
        //info!("update a block id from {} to {}", old_id, new_id);
        //warn!("consider to increase slot time, block generation too fast");
        //match self.block_by.get(&old_id) {
            //None => panic!("cannot update block id,I don't have cmt block"),
            //Some(cmt_block) => {
                //self.block_by.insert(new_id, cmt_block.clone());
                //self.block_by.remove(&old_id);
            //}
        //}
    //}

    // currently a hack, need to combine with sample_cmt
    //pub fn get_cmt_header(&self, block_id: u64) -> BlockHeader {
        //match &self.block_by.get(&block_id) {
            //None => panic!("I don't have cmt block"),
            //Some(cmt_block) => cmt_block.block_header.clone(),
        //}
    //}

    pub fn package_trans(&mut self, transactions: &mut Vec<Transaction>) {
        let tx_bytes_size = self.transaction_size_in_bytes();
        if tx_bytes_size > self.block_size {
            let mut s = 0;
            for i in 0..self.transactions.len() {
                s += self.transactions[i].bytes().len();
                if s > self.block_size {
                    if self.transactions.len() == 0 {
                        panic!("single transaction too large, block size is insufficient");
                    }
                    break;
                } else {
                    transactions.push(self.transactions[i].clone());
                }
            }
            for _ in 0..transactions.len() {
                self.transactions.pop_front();
            }
        } else {
            for tx in &self.transactions {
                transactions.push(tx.clone());
            }
            self.transactions.clear();
        }
        //let mut trans_byte = transactions.iter().map(Transaction::bytes).collect::<Vec<Bytes>>();
        //let mut total_size = 0;
        //for tx in &trans_byte {
            //total_size +=  tx.len();
        //}
    }

    pub fn prepare_transaction_block(&mut self) -> Vec<Transaction>{
        let mut transactions: Vec<Transaction> = Vec::new();
        self.package_trans(&mut transactions);
        transactions
    }

    //pub fn prepare_cmt_block(&mut self, block_id: u64) -> Option<BlockHeader> {
        //let mut rng = rand::thread_rng();

        //// get CMT
        //let header = BlockHeader {
            //version: 1,
            //previous_header_hash: CMTH256::default(),
            //merkle_root_hash: CMTH256::default(),
            //time: 4u32,
            //bits: 5.into(),
            //nonce: rng.gen(),
            //coded_merkle_roots_hashes: vec![CMTH256::default(); 8],
        //};
        //// CMT - propose block
        //// let transaction_size = Transaction::bytes(&self.transactions[0]).len();
        //// info!("{:?} transaction_size {:?}", self.addr, transaction_size);

        //let mut transactions: Vec<Transaction> = Vec::new();
        //self.package_trans(&mut transactions);
        //info!("num trans in block {}", transactions.len());

        //let start = SystemTime::now();
        //// autopad transactions
        //let (block, trans_len) = CMTBlock::new(
            //header.clone(), 
            //&transactions, 
            //BLOCK_SIZE as usize, 
            //HEADER_SIZE, 
            //&self.codes_for_encoding, 
            //vec![true; self.codes_for_encoding.len()]
        //);
        ////PERFORMANCE_COUNTER.record_generated_transaction();

        //let cmt_header = block.block_header.clone();
        ////self.block_by.insert(block_id, block);

        //return Some(cmt_header);
    //}

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn remove_block(&mut self, block_id: u64) {
        info!("mempool remove {}", block_id);
        self.symbols_by.remove(&block_id);
        self.headers_by.remove(&block_id);
    }

    
   pub fn insert(&mut self, transaction: Transaction) {
        self.transactions.push_back(transaction);
        let tx_bytes_size = self.transaction_size_in_bytes();

        // need to truncate 
        if tx_bytes_size > 0 {//self.block_size {
            self.schedule_handler.send(scheduler::Signal::Control);
        }
    }


    pub fn insert_transactions(&mut self, transactions: Vec<Transaction>) {
        self.transactions.extend(transactions);
        let tx_bytes_size = self.transaction_size_in_bytes();
        //info!("tx_bytes_size {} num {}", tx_bytes_size, self.transactions.len());
        if tx_bytes_size > self.block_size {
            self.schedule_handler.send(scheduler::Signal::Control);
        }
    }

    pub fn estimate_gas(&mut self, transaction: Transaction) {
    }
}



    //pub fn prepare_block(&mut self) -> Option<BlockHeader> {
        //if self.transactions.len() == 0 {
            //return None;
        //}

        //let transaction_size = Transaction::bytes(&self.transactions[0]).len();
        //info!("{:?} transaction_size {:?}", self.addr, transaction_size);

        //let mut transactions: Vec<Transaction> = Vec::new();
        //let mut transactions_bytes: Vec<Vec<u8>> = Vec::new();
        //let tx_bytes_size = self.transaction_size_in_bytes();

        //let start = SystemTime::now();
        //// need to truncate 
        //if tx_bytes_size > self.block_size {
            //let mut s = 0;
            //for i in 0..self.transactions.len() {
                //let trans_byte = self.transactions[i].bytes();
                //s += trans_byte.len();
                //if s > self.block_size {
                    //if self.transactions.len() == 0 {
                        //panic!("single transaction too large, block size is insufficient");
                    //}
                    //break;
                //} else {
                    //transactions.push(self.transactions[i].clone());
                    //transactions_bytes.push(trans_byte.to_vec());
                //}
            //}

            //for _ in 0..transactions.len() {
                //self.transactions.pop_front();
            //}
        //} else {
            //for tx in &self.transactions {
                //transactions.push(tx.clone());
                //transactions_bytes.push(tx.bytes().to_vec());
            //}
            //self.transactions.clear();

        //}

        //let mut trans_byte = transactions.iter().map(Transaction::bytes).collect::<Vec<Bytes>>();
        //info!("{:?} copied  block {:?}", self.addr, start.elapsed()); 

        //let merkle_tree = MerkleTree::from_vec(algorithm, transactions_bytes);
        //info!("{:?} prepared merkle_tree {:?}", self.addr, start.elapsed()); 

        //let root_hash = merkle_tree.root_hash();

        //let cmt_root_hash: CMTH256 = root_hash[..].into();
        ////info!("root_hash {:?}", root_hash);
        ////let proof = merkle_tree.gen_proof(vec![0]).unwrap();

        //let mut rng = rand::thread_rng();
        //let header = BlockHeader {
            //version: 1,
            //previous_header_hash: CMTH256::default(),
            //merkle_root_hash: cmt_root_hash,
            //time: 4u32,
            //bits: 5.into(),
            //nonce: rng.gen(),
            //coded_merkle_roots_hashes: vec![CMTH256::default(); 32],
            //delimitor: vec![],
        //};
        ////
        ////let f = File::open("t").expect("Unable to open file");
        ////let f = BufReader::new(f);
        ////let mut header_hex = "".to_string();
        ////for line in f.lines() {
            ////header_hex = line.expect("Unable to read line").to_string();
        ////}

        ////let header_bytes = hex::decode(&header_hex).unwrap();
        ////let header: BlockHeader = deserialize(&header_bytes as &[u8]).unwrap();
        //let block = CMTBlock {
            //block_header: header.clone(),
            //transactions: transactions,
            //coded_tree: vec![], //Coded Merkle tree constructed from the transactions in the block
            //block_size_in_bytes: 65535, // size of transactions in the block, used to specify block size for tests
        //};

        //self.cmt_block = Some(block);
        //return Some(header);
    //}

