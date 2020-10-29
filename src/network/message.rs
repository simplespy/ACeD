use serde::{Serialize, Deserialize};
use mio_extras::channel::{self, Sender};
use std::sync::mpsc::{self};
use super::primitive::block::{EthBlkTransaction};
use chain::transaction::Transaction;
use super::scheduler::Token;
use std::net::{SocketAddr};
use chain::{BlockHeader}; 
use super::cmtda::{Block, H256, BLOCK_SIZE, HEADER_SIZE, read_codes};
use ser::{deserialize, serialize};
use primitives::bytes::{Bytes};
use chain::decoder::{Symbol};
use chain::big_array::{BigArray};
use super::primitive::block::ContractState;
use web3::types::Address;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Samples {
    pub header: Vec<u8>,
    pub symbols: Vec<Vec<Symbol>>,
    pub idx: Vec<Vec<u64>>,
}

impl Samples {
    pub fn merge(&mut self, samples: &Samples) -> bool {
        if self.header != samples.header {
            return false;
        }

        // has the same number of layers
        let num_layer = self.symbols.len();
        if (samples.symbols.len() != num_layer) || 
           (samples.idx.len() != num_layer) {
            return false;
        }

        // TODO check if they are redundant 
       
        // merge
        for i in 0..num_layer {

        }
        return true;
    }
}

// prototype only, message can be made secured with crypto
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Ping(String),
    Pong(String),
    SyncBlock(EthBlkTransaction),
    SendTransaction(Vec<u8>), 
    PassToken(Token),
    //ip(pubkey) BlockHeader block_id //sender is client
    ProposeBlock(SocketAddr, u64, Vec<u8>), 
    ScaleReqChunks(SocketAddr, u64, u64), //(id, scale_id), // sender is scalenode
    ScaleReqChunksReply(SocketAddr, u64, Samples),
    MySign(String, u64, u64, String, String, u64),
    ScaleGetAllChunks(ContractState), // blockheader
    ScaleGetAllChunksReply((Option<Samples>, u64)),
}


#[derive(Debug, Clone)]
pub struct ConnectHandle {
    pub result_sender: mpsc::Sender<ConnectResult>,
    pub dest_addr: SocketAddr,
}

#[derive(Debug, Clone)]
pub enum ServerSignal {
    ServerConnect(ConnectHandle),
    ServerDisconnect, 
    ServerStop,
    ServerStart,
    ServerBroadcast(Message),
    ServerUnicast((SocketAddr, Message)),
}

#[derive(Clone)]
pub struct PeerHandle {
    pub write_queue: channel::Sender<Vec<u8>>,   
    pub addr: SocketAddr,
}

impl PeerHandle {
    pub fn write(&self, msg: Message) {
        let buffer = bincode::serialize(&msg).unwrap();
        if self.write_queue.send(buffer).is_err() {
            warn!("Failed to send write request for peer {}, channel detached", self.addr);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectResult {
    Success,
    Fail,
}

pub struct TaskRequest {
    pub peer: Option<PeerHandle>,
    pub msg: Message,
}
