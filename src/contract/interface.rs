use crossbeam::channel::{Sender};
use super::primitive::block::{EthBlkTransaction, ContractState, Block};
use web3::types::{Address, H256, TransactionReceipt, U256};

#[derive(Clone)]
pub struct Handle {
    pub message: Message,
    pub answer_channel: Option<Sender<Answer>>,
}
#[derive(Clone)]
pub enum Response {
    SendBlock,
    GetCurrState(ContractState),
    CountScaleNode(usize), 
    AddScaleNode,
    ScaleNodesList(Vec<Address>),
    TxReceipt(TransactionReceipt),
    GetAll(Vec<EthBlkTransaction>),
    SyncChain(usize),
}
#[derive(Clone)]
pub enum Answer {
    Success(Response),
    Fail(String),
}
#[derive(Clone)]
pub enum Message {
    SendBlock(Block),
    GetCurrState(usize),
    CountScaleNodes,
    AddScaleNode(String, String),
    GetScaleNodes,
    GetTxReceipt(H256),
    GetAll(([u8;32], usize, usize)), //inithash, start, end
    SyncChain,
    EstimateGas(Block),
    SubmitVote(String, U256, U256, U256, U256, U256),
    ResetChain(usize),
    AddSideNode(usize),
}

pub enum Error {
    TimeOut,
    ConnectionFail,
}
