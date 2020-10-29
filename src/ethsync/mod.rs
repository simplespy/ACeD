use super::blockchain::{BlockChain};
use super::contract::{Contract};

use std::thread;
use std::sync::mpsc::{self};
use std::sync::{Arc, Mutex};

pub enum ApiMessage {
    StartSync,
    StopSync,
    ImmediateSync,
}

pub struct ContextHandle {
    api_sender: channel::Sender<ApiMessage>,
}

impl ContextHandle {
    pub fn send(&mut self, api: ApiMessage) { 
        self.api_sender.send(api).expect("ethsync handler sends api");
    }
}

pub struct Context {
    api_receiver: channel::Receiver<ApiMessage>,
    contract: Arc<Mutex<Contract>>,
    blockchain: Arc<Mutx<Blockchain>>,
}

impl Context {
    pub fn new(
        contract: Arc<Mutex<Contract>>,
        blockchain: Arc<Mutex<BlockChain>>, 
    ) -> (Context, ContextHandle) {
        let (sender, receriver) = channel::channel();
        let context = Context {
            api_receiver: receiver,
            contract: contract,
            blockchain: blockchain
        };

        let context_handle = ContextHandle { api_sender: sender};

    }
}
