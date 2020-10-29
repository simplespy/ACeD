extern crate tiny_http;

use super::{TxGenSignal};
use super::mempool::mempool::{Mempool};
use super::blockchain::blockchain::BlockChain;
use super::db::blockDb::BlockDb;
use super::contract::interface::{Message, Handle, Answer};
use super::contract::interface::Response as ContractResponse;
use crossbeam::channel::{self, Sender};
use std::thread;
use tiny_http::{Server, Response, Header};
use url::Url;
use std::net::{SocketAddr};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap};
use serde::{Serialize};
use crate::network::message::{PeerHandle, ServerSignal};
use super::network::message::Message as PerformerMessage;
use super::experiment::snapshot::{PERFORMANCE_COUNTER};
use mio_extras::channel::Sender as MioSender;
use web3::types::U256;

pub struct ApiServer {
    addr: SocketAddr,
    tx_gen_control: Sender<TxGenSignal>,
    contract_channel: Sender<Handle>,
}

pub struct RequestContext {
    tx_control: Sender<TxGenSignal>,
    mempool: Arc<Mutex<Mempool>>,
    chain: Arc<Mutex<BlockChain>>,
    block_db: Arc<Mutex<BlockDb>>,
    contract_channel: Sender<Handle>,
    server_control: MioSender<ServerSignal>,
}

#[derive(Serialize)]
pub struct ApiResponse {
    success: bool,
    message: String,
}

macro_rules! respond_result {
    ( $req:expr, $success:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let api_result = ApiResponse {
            success: $success,
            message: $message.to_string(),
        };
        let response = Response::from_string(serde_json::to_string_pretty(&api_result).unwrap())
            .with_header(content_type);
        $req.respond(response).unwrap();
    }};
}



impl ApiServer {
    pub fn start(socket: SocketAddr, 
                 tx_control: Sender<TxGenSignal>, 
                 mempool: Arc<Mutex<Mempool>>,
                 contract_channel: Sender<Handle>,
                 chain: Arc<Mutex<BlockChain>>,
                 block_db: Arc<Mutex<BlockDb>>,
                 server_control: MioSender<ServerSignal>,
    ) {
        let server = Server::http(&socket).unwrap();
        let _handler = thread::spawn(move || {
            for request in server.incoming_requests() {
                let rc = RequestContext {
                    tx_control: tx_control.clone(),
                    mempool: mempool.clone(),
                    chain: chain.clone(),
                    block_db: block_db.clone(),
                    contract_channel: contract_channel.clone(),
                    server_control: server_control.clone(),
                };
                // new thread per request
                let _ = thread::spawn(move || {
                    let url_path = request.url();
                    let mut url_base = Url::parse(&format!("http://{}/", &socket)).expect("get url base");
                    let url = url_base.join(url_path).expect("join url base and path");
                    
                    match url.path() {
                        "/server/ping" => {
                            let text = "hello Ping from ".to_owned() + &socket.ip().to_string();
                            let signal = ServerSignal::ServerBroadcast(PerformerMessage::Ping(text));
                            rc.server_control.send(signal);
                            respond_result!(request, true, "ok");
                        },


                        "/telematics/snapshot" => {
                            let snapshot = PERFORMANCE_COUNTER.snapshot();
                            let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
                            let response = Response::from_string(serde_json::to_string(&snapshot).unwrap()).with_header(content_type);
                            request.respond(response);
        
                        },
                        "/transaction-generator/start" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let interval = match pairs.get("interval") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing step");
                                    return;
                                },
                            };
                            let s = match interval.parse::<usize>() {
                                Ok(s) => s,
                                Err(_) => {
                                    respond_result!(request, false, "step needs to be numeric");
                                    return;
                                },
                            };
                            rc.tx_control.send(TxGenSignal::Start(s as u64));
                            respond_result!(request, true, "ok");
                        },
                        "/transaction-generator/stop" => {
                            rc.tx_control.send(TxGenSignal::Stop);
                        },
                        "/transaction-generator/step" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let step = match pairs.get("step") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing step");
                                    return;
                                },
                            };
                            let step = match step.parse::<usize>() {
                                Ok(s) => s,
                                Err(_) => {
                                    respond_result!(request, false, "step needs to be numeric");
                                    return;
                                },
                            };
                            rc.tx_control.send(TxGenSignal::Step(step));
                            respond_result!(request, true, "ok");
                        },
                        "/transaction-generator/simulate" => {
                            rc.tx_control.send(TxGenSignal::Simulate);
                            respond_result!(request, true, "ok");
                        },
                        "/blockchain/get-curr-state" => {
                            println!("before /blockchain/get-curr-state lock" );
                            let chain = rc.chain.lock().expect("api get-curr-state");
                            println!("after /blockchain/get-curr-state lock" );
                            let state = chain.get_latest_state().expect("/blockchain/get-curr-state empty");
                            drop(chain);
                            respond_result!(request, true, format!("{:?}, {}", state.block_id, state.curr_hash.to_string()));
                            //respond_result!(request, true, format!("{:?}", state));
                        },
                        "/block-db/get-stored-blocks" => {
                            let block_db = rc.block_db.lock().expect("api gets block db");
                            let num_blocks = block_db.get_num_blocks();
                            drop(block_db);
                            respond_result!(request, true, format!("{:?}", num_blocks));
                        },
                        "/block-db/set-block-thresh" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let thresh = match pairs.get("thresh") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing size");
                                    return;
                                },
                            };
                            let thresh = match thresh.parse::<usize>() {
                                Ok(s) => s,
                                Err(_) => {
                                    respond_result!(request, false, "need to be numeric");
                                    return;
                                },
                            };
                            let mut  block_db = rc.block_db.lock().expect("gets block db");
                            block_db.thresh = thresh;
                            drop(block_db);
                            respond_result!(request, true, "ok");
                        },
                        "/mempool/change-size" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let size = match pairs.get("size") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing size");
                                    return;
                                },
                            };
                            let size = match size.parse::<usize>() {
                                Ok(s) => s,
                                Err(_) => {
                                    respond_result!(request, false, "size need to be numeric");
                                    return;
                                },
                            };
                            let mut mempool = rc.mempool.lock().expect("api change mempool size");
                            mempool.change_mempool_size(size);
                            drop(mempool);
                            respond_result!(request, true, format!("mempool size changed to {}", size));
                        },
                        "/mempool/num-transaction" => {
                            let mut mempool = rc.mempool.lock().expect("api change mempool size");
                            let num = mempool.get_num_transaction();
                            drop(mempool);
                            respond_result!(request, true, &num.to_string());
                        },
                        "/contract/get-tx-receipt" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let hash = match pairs.get("hash") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing hash");
                                    return;
                                },
                            };
                            let a = hex::decode(hash).unwrap();
                            let tx_hash: &[u8] = a.as_ref();
                            let tx_hash = web3::types::H256::from_slice(tx_hash);

                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::GetTxReceipt(tx_hash),
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let receipt = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::TxReceipt(receipt) => receipt,
                                                _ => {
                                                    panic!("answer to GetScaleNodes: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{:?}", receipt));
                        },
                        "/contract/reset-chain" => {
                            info!("reset-chain");
                            let handle = Handle {
                                message: Message::ResetChain(0),
                                answer_channel: None,
                            };
                            rc.contract_channel.send(handle);
                        }
                        "/contract/count-scale-nodes" => {
                            // USE CALLBACK
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::CountScaleNodes,
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let num_node = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::CountScaleNode(num_scale_node) => num_scale_node,
                                                _ => {
                                                    panic!("answer to NumScaleNode: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{}", num_node));
                        },
                        "/contract/get-curr-state" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::GetCurrState(0),
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let curr_state = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::GetCurrState(curr_state) => curr_state,
                                                _ => {
                                                    panic!("answer to GetCurrState: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{:?}, {:?}", curr_state.block_id, curr_state.curr_hash));
                        },
                        "/contract/get-scale-nodes" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::GetScaleNodes,
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let scale_nodes = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::ScaleNodesList(scale_nodes) => scale_nodes,
                                                _ => {
                                                    panic!("answer to GetScaleNodes: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{:?}", scale_nodes));
                        },
                        "/contract/add-scale-node" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let id = match pairs.get("id") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing id");
                                    return;
                                },
                            };
                            let ip = match pairs.get("ip") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing ip");
                                    return;
                                },
                            };
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            //let id = id.parse().unwrap();
                            let handle = Handle {
                                message: Message::AddScaleNode(id.clone(), ip.clone()),
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let reply = Response::from_string(format!("Add scaleNode {}", id));
                            request.respond(reply);
                        },
                        "/contract/submit-vote" => {

                            //let (answer_tx, answer_rx) = channel::bounded(1);
                            //let header = super::contract::utils::_generate_random_header();
                            //let (sigx, sigy) = super::contract::utils::_sign_bls(header.clone(), "node1".to_string());
                            //let (sigx2, sigy2) = super::contract::utils::_sign_bls(header.clone(), "node2".to_string());
                            //let (sigx3, sigy3) = super::contract::utils::_sign_bls(header.clone(), "node3".to_string());
                            //let (sigx, sigy) = super::contract::utils::_aggregate_sig(sigx, sigy, sigx2, sigy2);
                            //let (sigx, sigy) = super::contract::utils::_aggregate_sig(sigx, sigy, sigx3, sigy3);
                            ////  self.submit_vote(header, U256::from_dec_str(sigx.as_ref()).unwrap(), U256::from_dec_str(sigy.as_ref()).unwrap(), U256::from(26))
                            //let handle = Handle {
                                //message: Message::SubmitVote("deadbeef".to_string(), U256::from(0), U256::from(5), U256::from_dec_str(sigx.as_ref()).unwrap(), U256::from_dec_str(sigy.as_ref()).unwrap(), U256::from(26)),
                                //answer_channel: Some(answer_tx),
                            //};
                            //rc.contract_channel.send(handle);
                        },
                        "/contract/get-all" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::GetAll(([0u8; 32], 0, 0)),
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                        },
                        "/contract/sync-chain" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::SyncChain,
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let chain_len = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::SyncChain(chain_len) => chain_len,
                                                _ => {
                                                    panic!("answer to GetScaleNodes: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{:?}", chain_len));
                        },
                        "/contract/add-side-node" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::AddSideNode(0),
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let chain_len = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::SyncChain(chain_len) => chain_len,
                                                _ => {
                                                    panic!("answer to GetScaleNodes: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                        },
                        _ => {
                            println!("all other option {:?}", url.path());
                        }
                    }

                    
                });

                
            }     
        });
        info!("API server listening");
    }
}


