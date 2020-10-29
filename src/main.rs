#[macro_use]
extern crate clap;

use std::{thread, time};
use std::sync::mpsc::{self};
use mio_extras::channel::{self};
use clap::{Arg, App, SubCommand, ArgMatches};
use std::fs::File;
use std::io::{BufRead, BufReader};
use system_rust::network::message::{ServerSignal, ConnectResult, ConnectHandle, Message};
use system_rust::network::performer;

use system_rust::network::server;
use system_rust::mempool::scheduler::{Scheduler, Token};
use system_rust::db::blockDb::{BlockDb};
use system_rust::blockchain::blockchain::{BlockChain};
use system_rust::mempool::mempool::{Mempool};
use system_rust::contract::contract::{Contract, Account};
use std::sync::{Arc, Mutex};
use system_rust::api::apiServer::ApiServer;
use system_rust::experiment::transactionGenerator::{TransactionGenerator};
use std::net::{SocketAddr};
use crossbeam::channel as cbchannel;
use log::{info, warn, error, debug};
use system_rust::mainChainManager::{Manager};
use system_rust::cmtda::{read_codes};
use chain::decoder::{Code};
use system_rust::contract::interface::{Handle, Answer};
use system_rust::contract::interface::Message as ContractMessage;
use system_rust::contract::interface::Response as ContractResponse;
use system_rust::contract::utils::{BLSKey, BLSKeyStr};
use system_rust::primitive::block::{ContractState};
use web3::types::Address;
use system_rust::experiment::snapshot::PERFORMANCE_COUNTER;

fn main() {
    env_logger::init();
    let matches = clap_app!(myapp =>
        (version: "0.0")
        (author: "Anonymous Submitter")
        (about: "simple blockchain network")
        (@arg known_peer: -c --connect ... [PEER] "Sets ip to connect to")
        (@arg side_node: -r --side_node ... [SIDE] "Sets side ip to connect to")
        (@arg peer_addr: -i --p2p [ADDR]  "Sets ip to listen")
        (@arg api_addr: -a --api_addr [ADDR] "Sets port for api")
        (@arg account: -d --account  [ACCOUNT] "Sets account address")
        (@arg contract_addr: -f --contract_addr [ADDR] "Sets ETH contract address")
        (@arg node_url: -u --node_url [HTTP] "Sets ETH node https url")
        (@arg key: -k --key  +takes_value "Sets key address")
        //(@arg has_token: -t --has_token "Sets init token")
        (@arg scale_id: -s --scale_id  +takes_value "Sets scalechain node")
        (@arg ldpc: -l --ldpc  +takes_value "get ldpc file path")
        (@arg num_scale: -n --num_scale +takes_value "get number scale node")
        (@arg binary_dir: -b --binary_dir +takes_value "get bls binary")
        (@arg abi_path: -j --abi_path +takes_value "get api_path")
        (@arg num_side: -e --num_side +takes_value "get num side")
        (@arg slot_time: -t --slot_time +takes_value "get slot time")
        (@arg start_time: --start_time +takes_value "contract starting time, measured in UNIX EPOCH")
        (@subcommand addScaleNode =>
            (@arg contract_addr: -f --contract_addr [ADDR] "Sets ETH contract address")
            (@arg node_url: -u --node_url [HTTP] "Sets ETH node https url")
            (@arg account: --account [ACCOUNT] "get account file")
            (@arg new_account: --new_account +takes_value "get account file")
            (@arg keyfile: --keyfile +takes_value "get key file")
            (@arg ip_addr: --ip_addr +takes_value "get ip_addr")
        )
        (@subcommand getCurrState =>
            (@arg account: --account [ACCOUNT]  "get account file")
            (@arg contract_addr: -f --contract_addr [ADDR] "Sets ETH contract address")
            (@arg node_url: -u --node_url [HTTP] "Sets ETH node https url")
        )
        (@subcommand resetChain =>
            (@arg account: --account [ACCOUNT]  "get account file")
            (@arg contract_addr: -f --contract_addr [ADDR] "Sets ETH contract address")
            (@arg node_url: -u --node_url [HTTP] "Sets ETH node https url")
        )
        (@subcommand getScaleNodes =>
            (@arg account: --account [ACCOUNT]  "get account file")
            (@arg contract_addr: -f --contract_addr [ADDR] "Sets ETH contract address")
            (@arg node_url: -u --node_url [HTTP] "Sets ETH node https url")
        )
    )
    .get_matches();

    match matches.subcommand() {
        ("addScaleNode", Some(m)) => {
            let contract = get_contract_instance(&m);
            let account: Account = match m.value_of("new_account") {
                Some(account_path) => {
                    let file = File::open(account_path).unwrap();
                    serde_json::from_reader(file).expect("deser account")
                },
                None => panic!("unable to locate account"),
            };
            let key_path = m.
                value_of("keyfile").
                expect("missing key file");           
            let key_file = File::open(key_path).unwrap();
            let key_str: BLSKeyStr = match serde_json::from_reader(key_file) {
                Ok(k) => k,
                Err(e) => {
                    error!("unable to deser keyfile {:?}", key_path);
                    return;
                }
            };
            let key: BLSKey = BLSKey::new(key_str);
            let ip_addr = m.
                value_of("ip_addr").
                unwrap().
                to_string();
            info!("get scale id {:?}", account.address);
            match contract._get_scale_id(account.address.clone()) {
                Some(i) => {
                    if i.as_usize() == 0 {
                        contract.add_scale_node(
                            account.address,
                            ip_addr,
                            key.pkx1, key.pkx2, 
                            key.pky1, key.pky2
                        );
                    }
                },
                None => {
                    contract.add_scale_node(
                        account.address,
                        ip_addr,
                        key.pkx1, key.pkx2, 
                        key.pky1, key.pky2
                    );
                    println!("Registered Address");
                }
            }
            return;
        },
        ("getCurrState", Some(m)) => {
            let contract = get_contract_instance(&m);
            let state = contract._get_curr_state(0); 
            println!("hash: {:?}\nblock_id: {:?}", state.curr_hash, state.block_id);
            return;
        },
        ("resetChain", Some(m)) => {
            let contract = get_contract_instance(&m);
            let mut state = contract._get_curr_state(0); 
            if state.block_id != 0 {
                contract.reset_chain(0); 
                state = contract._get_curr_state(0); 
            }
            println!("hash: {:?}\nblock_id: {:?}", state.curr_hash, state.block_id);
            assert!(state.block_id==0);
            return;
        },
        ("getScaleNodes", Some(m)) => {
            let contract = get_contract_instance(&m);
            let num_scale = contract._count_scale_nodes(); 
            println!("{:?}", num_scale);
            let mut scale_list = vec![];
            let mut scale_pub = vec![];
            // the node 0 is considered special for current contract design
            for i in 1..num_scale {
                scale_list.push(contract._get_scale_node(i));
            }

            for i in scale_list.iter() {
                scale_pub.push(contract._get_scale_pub_key(*i));
            }
            
            println!("num scale node(node 0 does not count): {}", num_scale-1);
            println!("{:?}", scale_list);
            println!("{:?}", scale_pub);
            return;
        }
        _ => {},
    }


    let p2p_addr = matches.
        value_of("peer_addr").
        unwrap().
        parse::<SocketAddr>().
        unwrap_or_else(|e| {
            panic!("Error parsing p2p server address");
        });

    let contract_addr: Address = matches.
        value_of("contract_addr").
        expect("missing key file").
        parse().
        unwrap();

    let rpc_url = matches.
        value_of("node_url").
        expect("missing url link");

    let api_socket = matches.
        value_of("api_addr").
        unwrap().
        parse::<SocketAddr>().
        unwrap_or_else(|e| {
            panic!("Error parsing api server address");
        });



    let bin_path = matches.value_of("binary_dir").expect("missing binary path");
    let abi_path = matches.value_of("abi_path").expect("missing json abi path");
    let key_path = matches.value_of("key").expect("missing key file");
    let ldpc_path = matches.value_of("ldpc").expect("missing ldpc file");
    let mut scale_id: u64 = matches.value_of("scale_id").expect("missing scaleid").parse::<u64>().unwrap();
    let mut num_scale: u64 = matches.value_of("num_scale").expect("missing number of scale").parse::<u64>().unwrap();
    let mut slot_time: f32 = matches.value_of("slot_time").expect("missing slot time").parse::<f32>().unwrap();
    let mut start_time: f64 = matches.value_of("start_time").expect("missing starting time").parse::<f64>().unwrap();
    let start_sec: u64 = start_time.floor() as u64;
    let start_millis: u64 = ((start_time - start_time.floor())*1000.0).floor() as u64;

    PERFORMANCE_COUNTER.record_scale_id(scale_id as usize);

    info!("sec    {}", start_sec);
    info!("millis {}", start_millis);

    // get neighnors
    let mut neighbors = vec![];
    if let Some(known_peers) =  matches.values_of("known_peer") {
        let known_peers: Vec<String> = known_peers.map(|x| x.to_owned()).collect();
        for peer in known_peers {
            match peer.parse::<SocketAddr>() {
                Ok(addr) => neighbors.push(addr),
                Err(_) => panic!("parse peer addr error"),
            }
        }
    }

    let mut sidenodes = vec![];
    if let Some(side_nodes) =  matches.values_of("side_node") {
        let side_nodes: Vec<String> = side_nodes.map(|x| x.to_owned()).collect();
        for peer in side_nodes {
            match peer.parse::<SocketAddr>() {
                Ok(addr) => sidenodes.push(addr),
                Err(_) => panic!("parse peer addr error"),
            }
        }
    }

    let num_side = sidenodes.len() as u64;


    //let has_token = sidenodes[0] == p2p_addr;

    //println!("side_nodes {:?}", sidenodes);

    let is_scale_node: bool = (scale_id > 0);
    
    // get accounts
    info!("api socket {:?}", api_socket);
    let account: Account = match matches.value_of("account") {
        Some(account_path) => {
            let file = File::open(account_path).unwrap();
            serde_json::from_reader(file).expect("deser account")
        },
        None => panic!("unable to locate account"),
    };

    let key_file = File::open(key_path).unwrap();
    let key_str: BLSKeyStr = match serde_json::from_reader(key_file) {
        Ok(k) => k,
        Err(e) => {
            error!("unable to deser keyfile {:?}", key_path);
            return;
        }
    };
    let key: BLSKey = BLSKey::new(key_str);
    

    // roles
    let block_db_path = "/tmp/db".to_owned() + &matches.
        value_of("peer_addr").
        unwrap();
    let block_db = Arc::new(Mutex::new(BlockDb::new(block_db_path)));
    let blockchain = Arc::new(Mutex::new(BlockChain::new()));

    let (task_sender, task_receiver) =cbchannel::unbounded();

    let (server_ctx, mut server_handle) = server::Context::new(
        task_sender.clone(), 
        p2p_addr,
        is_scale_node,
    );
    server_ctx.start();

    let (schedule_handle_sender, schedule_handle_receiver) = cbchannel::unbounded();
    let (contract_handle_sender, contract_handle_receiver) = cbchannel::unbounded();
    let (manager_handle_sender, manager_handle_receiver) = cbchannel::unbounded();
    let k_set: Vec<u64> = vec![128,64,32,16,8,4];//vec![32,16,8];//  //128,64,32,16,8
    let (codes_for_encoding, codes_for_decoding) = read_codes(k_set.clone(), ldpc_path);
    let mempool = Arc::new(Mutex::new(Mempool::new(
        contract_handle_sender.clone(),
        schedule_handle_sender.clone(),
        p2p_addr.clone(),
        codes_for_encoding.clone(),
        codes_for_decoding.clone(),
    )));

    

    //let token = init_token(has_token, p2p_addr.clone(), &sidenodes);

    let contract = Contract::new(
        account.clone(),
        key,
        task_sender.clone(),
        server_handle.control_tx.clone(),
        contract_handle_receiver,
        p2p_addr.to_string(),
        abi_path.to_string(),
        rpc_url,
        &contract_addr,
    );

    let manager = Manager::new(
        contract_handle_sender.clone(),
        blockchain.clone(),
        mempool.clone(),
        server_handle.control_tx.clone(),
        p2p_addr.clone(),
        manager_handle_receiver,
        block_db.clone(),
        codes_for_encoding.clone(),
        codes_for_decoding.clone(),
        k_set.clone()
    );

    //if scale_id == 0 {
        //manager.start();
    //}

    let scheduler = Scheduler::new(
        p2p_addr.clone(), 
        None, 
        mempool.clone(), 
        server_handle.control_tx.clone(), 
        schedule_handle_receiver.clone(), 
        blockchain.clone(),
        contract_handle_sender.clone(),
        //side_id as u64,
        sidenodes.clone(),
        account.address.clone(),
        slot_time,
        start_sec,
        start_millis,
        num_scale,
        codes_for_encoding.clone(),
    );
    if scale_id == 0 {

        scheduler.start();
    }
    contract.start();

    // create main actors
    let mut performer = performer::new(
        task_receiver, 
        blockchain.clone(), 
        block_db.clone(),
        mempool.clone(),
        schedule_handle_sender.clone(),
        contract_handle_sender.clone(),
        p2p_addr.clone(),
        key_path.to_string(),
        scale_id,
        0,
        server_handle.control_tx.clone(),
        manager_handle_sender.clone(),
        num_scale,
        bin_path,
        num_side,
        account.address.clone(),
        slot_time,
        sidenodes.clone(),
        start_sec,
        start_millis,
    );
    performer.start();

    let (tx_gen, tx_control_sender) = TransactionGenerator::new(mempool.clone());
    tx_gen.start();

    ApiServer::start(
        api_socket, 
        tx_control_sender, 
        mempool.clone(), 
        contract_handle_sender.clone(), 
        blockchain.clone(),
        block_db.clone(),
        server_handle.control_tx.clone(),
    );

    let mut num_connected = 0;
    for neighbor in neighbors.iter() {
        let addr: SocketAddr = neighbor.to_string().parse().unwrap();
        loop {
            if addr == p2p_addr {
                break;
            }
            match server_handle.connect(addr) {
                Ok(rx) => {
                    match rx.recv() {
                        Ok(ConnectResult::Success) => {
                            info!("{:?} connected to {:?}", p2p_addr , addr);
                            break;
                        },
                        _ => (),//info!("{:?} unable to connect {:?}", p2p_addr , addr),
                    }
                    
                },
                Err(e) => {
                    error!(
                        "Error connecting to peer {}, retrying in one second: {}",
                        addr, e
                    );
                    thread::sleep(time::Duration::from_millis(1000));
                    continue;
                }
            }
        }
    }
    thread::park();
}

pub fn init_token(
    has_token: bool, 
    listen_socket: SocketAddr, 
    sidenodes: &Vec<SocketAddr>
) -> Option<Token> {
    let mut token: Option<Token> = None;
    if has_token {
        info!("creating token");
        let number_token = sidenodes.len();
        let mut tok = Token {
            version: 0,
            ring_size: 0,
            node_list: vec![],
        };
        for node in sidenodes.iter() {
            tok.ring_size += 1;
            tok.node_list.push(node.clone());
        }
        token = Some(tok);
    }
    token
}

pub fn parse_addr_file(filename: &str) -> Vec<SocketAddr> {
    let f = File::open(filename).expect("Unable to open file");
    let f = BufReader::new(f);

    let mut neighbors: Vec<SocketAddr> = vec![];
    for line in f.lines() {
        let addr = line.expect("file read error").parse().expect("unable to parse addr");
        neighbors.push(addr);
    }
    neighbors
}
pub fn sync_chain(contract_channel: cbchannel::Sender<Handle>) -> usize {
    let (answer_tx, answer_rx) = cbchannel::bounded(1);
    let handle = Handle {
        message: ContractMessage::SyncChain,
        answer_channel: Some(answer_tx),
    };
    contract_channel.send(handle);
    let chain_len = match answer_rx.recv() {
        Ok(answer) => {
            match answer {
                Answer::Success(response) => {
                    match response {
                        ContractResponse::SyncChain(chain_len) => chain_len,
                        _ => {
                            panic!("answer to GetMainNodes: invalid response type");
                        },
                    }
                },
                Answer::Fail(reason) => {
                    panic!("sync chain failure");
                },
            }
        },
        Err(e) => {
            panic!("main contract channel broke");
        },
    };   
    chain_len
}

pub fn get_contract_instance(m : &ArgMatches) -> Contract {
    let account: Account = match m.value_of("account") {
        Some(account_path) => {
            let file = File::open(account_path).unwrap();
            serde_json::from_reader(file).expect("deser account")
        },
        None => {
            let file = File::open("accounts/account1").unwrap();
            serde_json::from_reader(file).expect("deser account")
        }
    };
    let contract_addr = m.
        value_of("contract_addr").
        expect("missing contract file").
        parse().
        unwrap();
    let rpc_url = m.
        value_of("node_url").
        expect("missing url link");
    let contract = Contract::instance(
        &account,
        rpc_url,
        &contract_addr,
        );
    contract
}


