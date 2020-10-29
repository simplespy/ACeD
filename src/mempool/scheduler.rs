use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::net::{SocketAddr};
use super::mempool::{Mempool};
use super::message::{Message, ServerSignal};
use super::blockchain::{BlockChain};
use mio_extras::channel::Sender as MioSender;
use crossbeam::channel::{Receiver, Sender, self};
use std::{thread, time};
use super::cmtda::{BlockHeader, Block, H256, HEADER_SIZE, Transaction, read_codes};
use super::contract::utils;
use ser::{deserialize, serialize};
use super::contract::interface::{Handle, Answer};
use super::contract::interface::Message as ContractMessage;
use super::contract::interface::Response as ContractResponse;
use crypto::sha3::Sha3;
use crypto::digest::Digest;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use web3::types::Address;
use crate::experiment::snapshot::PERFORMANCE_COUNTER;
use chain::constants::{TRANSACTION_SIZE, BLOCK_SIZE, BASE_SYMBOL_SIZE, RATE, UNDECODABLE_RATIO};

use chain::decoder::{Code, Decoder, TreeDecoder, CodingErr, IncorrectCodingProof};
use chain::decoder::{Symbol};
use super::cmtda::Block as CMTBlock;
use super::cmtda::H256 as CMTH256;
use rand::Rng;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Token {
    pub version: usize,
    pub ring_size: usize,
    pub node_list: Vec<SocketAddr>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Signal {
    Data(Token),
    Control,
}

pub struct Scheduler {
    pub addr: SocketAddr, //p2p
    pub token: Option<Token>,
    pub mempool: Arc<Mutex<Mempool>>,
    pub server_control_sender: MioSender<ServerSignal>,
    pub contract_handler: Sender<Handle>,
    pub handle: Receiver<Signal>,
    pub chain: Arc<Mutex<BlockChain>>, 
    //pub side_id: u64,
    pub sidenodes: Vec<SocketAddr>,
    pub address: Address,
    pub slot_time: f32, 
    pub start_sec: u64, 
    pub start_millis: u64,
    pub prepared_block: Option<BlockHeader>,
    pub num_nodes: u64, //scale nodes
    pub symbols_by: Option<HashMap<u64, (Vec<Vec<Symbol>>, Vec<Vec<u64>>)>>,
    pub codes_for_encoding: Vec<Code>,
}

impl Scheduler {
    pub fn new(
        addr: SocketAddr,
        token: Option<Token>,
        mempool: Arc<Mutex<Mempool>>,
        server_control_sender: MioSender<ServerSignal>,
        handle: Receiver<Signal>,
        chain: Arc<Mutex<BlockChain>>,
        contract_handler: Sender<Handle>,
        //side_id: u64,
        sidenodes: Vec<SocketAddr>,
        address: Address,
        slot_time: f32,
        start_sec: u64,
        start_millis: u64,
        num_scale: u64,
        codes_for_encoding: Vec<Code>
    ) -> Scheduler {
        Scheduler {
            addr,
            token,
            mempool,
            server_control_sender,
            contract_handler,
            handle,
            chain: chain,
            //side_id,
            sidenodes,
            address,
            slot_time: slot_time,
            start_sec: start_sec,
            start_millis: start_millis,
            prepared_block: None,
            num_nodes: num_scale,
            symbols_by: None,
            codes_for_encoding: codes_for_encoding,
        }
    }

    // to participate a token ring group
    pub fn register_token(&mut self) -> bool {
        if let Some(ref mut token) = self.token {
            token.ring_size += 1;
            token.node_list.push(self.addr.clone());
            return true;
        } else {
            return false;
        }
    }

    

    //pub fn start(mut self) {
        //let _ = std::thread::spawn(move || {
            //loop {
                //match self.handle.recv() {
                    //Ok(v) => {
                        //match v {
                            //Signal::Control => {
                                //// try transmit
                                //match self.token.as_mut() {
                                    //None => (),
                                    //Some(token) => {
                                        ////info!("with token, propose a block");
                                        //self.propose_block();
                                    //},
                                //}
                            //},
                            //Signal::Data(token) => {
                                ////info!("reiceive a token, propose a block");
                                //self.token = Some(token);
                                //self.propose_block();
                            //}
                        //}
                    //},
                    //Err(e) => info!("scheduler error"),
                //}
            //}
        //});
    //}

    //pub fn get_time_diff() {
        //let curr_time: u64 = match time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            //Ok(n) => n.as_secs(),
            //Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        //};
         
    //}
    //

    pub fn get_side_id(&self) -> u64 {
        match self.sidenodes.
            iter().
            position(|&x| x== self.addr) 
        {
            Some(i) => i as u64,
            None => panic!("my socketaddr is not included in the side nodes ring"),
        }
    }

    pub fn start(mut self) {
        info!("scheduler started");
        let _ = std::thread::spawn(move || {
            loop {
                // setup
                let round = self.sidenodes.len() as u64;
                let round_millis = (self.slot_time * 1000.0) as u64 * round ;
                let side_id = self.get_side_id();     
                // pipelining
                match &self.prepared_block {
                    None => {
                        //info!("start preparing a block");
                        match self.prepare_block() {
                            None => thread::sleep(time::Duration::from_millis(100)),
                            _ => (),
                        }
                    }, 
                    Some(h) => { 
                        let (curr_slot, elapsed) = get_curr_slot(self.start_sec, self.start_millis, self.slot_time);

                        let curr_id = curr_slot % round;
                        // my slot
                        if curr_id == side_id {
                            PERFORMANCE_COUNTER.record_token_update(true);
                            if self.propose_block() {
                                let (curr_slot, elapsed) = get_curr_slot(self.start_sec, self.start_millis, self.slot_time);
                                // go over the deadline
                                if curr_slot%round != side_id {
                                    PERFORMANCE_COUNTER.record_token_update(false);
                                    continue;
                                } else {
                                    // to next slot
                                    let target_time = ((side_id+1) * (self.slot_time*1000.0) as u64)  - elapsed%round_millis;
                                    thread::sleep(time::Duration::from_millis(target_time));
                                    PERFORMANCE_COUNTER.record_token_update(false);
                                }
                            } 
                        } else if curr_id < side_id {
                            PERFORMANCE_COUNTER.record_token_update(false);
                            let target = side_id*(self.slot_time* 1000.0) as u64 - elapsed%round_millis;
                            thread::sleep(time::Duration::from_millis(target));
                        } else {
                            PERFORMANCE_COUNTER.record_token_update(false);
                            let time_left = round*(self.slot_time*1000.0) as u64 - elapsed%round_millis + side_id*(self.slot_time*1000.0) as u64;
                            thread::sleep(time::Duration::from_millis(time_left));
                        }
                    }
                }
            }
        });
    }

    //let curr_time = time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    //let round_time = self.slot_time* (self.sidenodes.len() as u64);
    //let curr_id = (curr_time.as_secs() % round_time) / self.slot_time;
    //let side_id = self.get_side_id();     
    //// my slot
    //if curr_id == side_id {
        //PERFORMANCE_COUNTER.record_token_update(true);
        //if self.propose_block() {
            //let curr_time = time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap(); 
            //let r_time = curr_time.as_secs() % round_time ;
            //let ceil_time = (( r_time/self.slot_time + 1) as u64) *self.slot_time;
            //if ceil_time <= r_time {
                //continue;
            //} else {
                //let n_time = (ceil_time - r_time as u64) * 1_000_000_000 - curr_time.subsec_nanos() as u64;
                //thread::sleep(time::Duration::from_nanos(n_time));
            //}
        //} else {
            //thread::sleep(time::Duration::from_millis(500));
        //}
    //} else if curr_id < side_id {
        //PERFORMANCE_COUNTER.record_token_update(false);
        //let time_left = ((side_id)*self.slot_time - (curr_time.as_secs() % round_time)) * 1_000_000_000 - curr_time.subsec_nanos() as u64;
        //let sleep_sec = time::Duration::from_nanos(time_left);
        //thread::sleep(sleep_sec);
    //} else {
        //PERFORMANCE_COUNTER.record_token_update(false);
        //let time_left = (round_time - (curr_time.as_secs() % round_time)+ side_id*self.slot_time) * 1_000_000_000 - curr_time.subsec_nanos() as u64;
        //let sleep_sec = time::Duration::from_nanos(time_left);
        //thread::sleep(sleep_sec);
    //}

    //fn pass_token(&mut self, token: Token) {
        //info!("{:?} passing token.", self.addr);
        //if token.ring_size >= 2 {
            //let mut index = 0;
            //for sock in &token.node_list {
                //if *sock == self.addr {
                    //let next_index = (index + 1) % token.ring_size;
                    //let next_sock = token.node_list[next_index];
                    //let message = Message::PassToken(token);
                    //let signal = ServerSignal::ServerUnicast((next_sock, message));
                    //self.server_control_sender.send(signal);
                    //break;
                //}
                //index = (index + 1) % token.ring_size;
            //}
        //} else {
            //let sleep_time = time::Duration::from_millis(1000);
            //thread::sleep(sleep_time);
            //self.propose_block();
        //}
    //}

    pub fn create_cmt_block(&mut self, trans: &Vec<Transaction>) -> Option<BlockHeader> {
        let mut rng = rand::thread_rng();
        let header = BlockHeader {
            version: 1,
            previous_header_hash: CMTH256::default(),
            merkle_root_hash: CMTH256::default(),
            time: 4u32,
            bits: 5.into(),
            nonce: rng.gen(),
            coded_merkle_roots_hashes: vec![CMTH256::default(); 8],
        };
        let (block, trans_len) = CMTBlock::new(
            header.clone(), 
            &trans, 
            BLOCK_SIZE as usize, 
            HEADER_SIZE, 
            &self.codes_for_encoding, 
            vec![true; self.codes_for_encoding.len()]
        );

        let cmt_header = block.block_header.clone();
        let num_symbol = BLOCK_SIZE/(BASE_SYMBOL_SIZE as u64) *((1.0/RATE) as u64);
        let mut symbols_by: HashMap<u64, (Vec<Vec<Symbol>>, Vec<Vec<u64>>)> = HashMap::new();

        // debug
        //let mut symbols = 
       
        for scale_id in (1..self.num_nodes+1) {
            //info!("scale_id {}", scale_id);
            let samples_idx = get_sample_index(
                scale_id, 
                num_symbol, 
                self.num_nodes); 
            let (mut symbols, mut idx) = block.sample_vec(samples_idx);
            // add sample, sample idx to mempool
            symbols_by.insert(scale_id, (symbols, idx));
        }

        //match decoder.run_tree_decoder(symbols.clone(), idx.clone(), cmt_block.block_header.clone()) {
            //Ok(transactions) => {
                //println!("transactions {:?}", transactions);

            //},
            //_ => info!("tree decoder error"),
        //};

        self.symbols_by = Some(symbols_by);
        self.prepared_block = Some(cmt_header);
        Some(header)
    }

    pub fn prepare_block(&mut self) -> Option<BlockHeader> {
        let tx_thresh = BLOCK_SIZE / TRANSACTION_SIZE - 1;
        // generate a coded block
        let mut mempool = self.mempool.lock().unwrap();
        let num_tx = mempool.get_num_transaction();
        if num_tx <= tx_thresh  { // transation size
            info!("{:?} Skip: {} less than {:?}", self.addr, num_tx, tx_thresh); 
            return None;
        }
        let trans = mempool.prepare_transaction_block();
        drop(mempool);
        let new_block_id = self.my_next_slot();
        PERFORMANCE_COUNTER.record_propose_block_update(new_block_id);
        let header = self.create_cmt_block(&trans);
        PERFORMANCE_COUNTER.record_propose_block_stop();
        header
    }

    pub fn propose_block(&mut self) -> bool {
        // construct message and broadcast 
        let (curr_slot, elapsed)= get_curr_slot(self.start_sec, self.start_millis, self.slot_time);
        let new_block_id =  curr_slot + 1; // a hack, to make sure curr_slot > 0, otherwise block rejected
        //info!("************start propose block with {}", new_block_id);
        PERFORMANCE_COUNTER.record_block_update(new_block_id);
        PERFORMANCE_COUNTER.record_propose_block_id(new_block_id as usize);
        
        let header = match &self.prepared_block {
            Some(header) => header.clone(),
            None => panic!("propose block without block ready"),
        };
        let mut mempool = self.mempool.lock().unwrap();
        let symbols = match self.symbols_by.take() {
            Some(s) => s,
            None => panic!("unable to take symbols in scheduler"),
        };
        mempool.insert_symbols(new_block_id, &header, symbols);
        drop(mempool);

        self.prepared_block = None;
        self.symbols_by = None;

        let header_bytes = serialize(&header);
        let header_message: Vec<u8> = header_bytes.clone().into();
        let hash_str = utils::hash_header_hex(&header_message);
        let message =  Message::ProposeBlock(
            self.addr, 
            new_block_id as u64, 
            header_message); 
        let signal = ServerSignal::ServerBroadcast(message);

        // last check before sending out the block
        let side_id = self.get_side_id();
        let (curr_slot, _) = get_curr_slot(self.start_sec, self.start_millis, self.slot_time); 
        let curr_id = curr_slot % self.sidenodes.len() as u64;
        if curr_id != side_id {
            info!("{:?} preempt take too long to construct block", self.addr);
            return false;
        }
        // send the block
        self.server_control_sender.send(signal);
        //PERFORMANCE_COUNTER.record_propose_block_stop();
        //let (curr_slot, elapsed) = get_curr_slot(self.start_sec, self.start_millis, self.slot_time);
        //info!("sent Propose_block {:?}", elapsed);
        true 
    }

    pub fn my_next_slot(&self) -> u64 {
        let (curr_slot, elapsed) = get_curr_slot(self.start_sec, self.start_millis, self.slot_time);
        let round = self.sidenodes.len() as u64;
        let curr_round = curr_slot / round;
        let side_id = self.get_side_id();
        let mut next_slot = curr_round * round + side_id;
        if next_slot <= curr_slot {
            next_slot += round;
        }
        next_slot
    }
}

// return slot and time elapsed as nano
// precision to millis, return curr_slot
pub fn get_curr_slot(start_sec: u64, start_millis: u64, slot_time: f32) -> (u64, u64) {
    let curr_time = time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let time_elapsed_millis = (curr_time.as_secs() - start_sec) *1000 + curr_time.subsec_millis() as u64 - start_millis;
    (time_elapsed_millis/((slot_time*1000.0) as u64), time_elapsed_millis)
}


// scale id starts at 1
pub fn get_sample_index(scale_id: u64, num_trans: u64, num_node: u64) -> Vec<u32> {
    let num_sample = ((num_trans as f32) / (num_node as f32)).ceil() as u64;
    let mut sample_idx = vec![];
    let start = (scale_id-1)*num_sample;
    let stop = if scale_id*num_sample > num_trans {
        num_trans
    } else {
        scale_id*num_sample
    };
    for i in start..stop {
        sample_idx.push(i as u32);
    }
    sample_idx
}
