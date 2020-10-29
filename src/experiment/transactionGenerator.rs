use super::mempool::mempool::Mempool;
use super::hash::{H256};
//use super::block::{Transaction, Input, Output};
use rand::rngs::ThreadRng;
use std::sync::{Mutex, Arc};
use std::thread;
use std::fs::File;
use std::io::{Write, BufReader, BufRead, Error};
use crossbeam::channel::{self, Sender, Receiver, TryRecvError};
use super::snapshot::PERFORMANCE_COUNTER;
use chain::transaction::{Transaction, TransactionInput, TransactionOutput, OutPoint};
use primitives::bytes::Bytes;
use rand::distributions::WeightedIndex;
use rand::distributions::Distribution;
use rand::seq::SliceRandom;
use std::time::{self, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

//use requests::{ToJson};
use rand::Rng;

pub enum TxGenSignal {
    Start(u64),
    Stop,
    Step(usize),
    Simulate,
}

pub enum State {
    Continuous(u64),
    Pause,
    Step(usize),
    Simulate,
}

pub enum  ArrivalDistribution {
    Uniform(UniformArrival),
}

pub struct UniformArrival {
    pub interval: u64, //us
}

pub struct TransactionGenerator {
    control: channel::Receiver<TxGenSignal>,
    mempool: Arc<Mutex<Mempool>>,
    my_addr: Vec<H256>,
    to_addr: Vec<H256>,
    state: State,
    total_tx: usize,
    arrival_distribution: ArrivalDistribution,
}

impl TransactionGenerator {
    pub fn new(mempool: Arc<Mutex<Mempool>>) -> (TransactionGenerator, Sender<TxGenSignal>) {
        let (tx, rx) = channel::unbounded();
        let transaction_gen = TransactionGenerator {
            mempool: mempool,
            my_addr: vec![H256::new()],
            to_addr: vec![H256::new()],
            control: rx,
            state: State::Pause,
            total_tx: 0,
            arrival_distribution: ArrivalDistribution::Uniform(UniformArrival { interval: 100 })
        };
        (transaction_gen, tx)
    }

    pub fn start(mut self) {
        let _ = thread::spawn(move || {
            loop {
                let tx_gen_start = time::Instant::now();
                match self.state {
                    State::Pause => {
                        let signal = self.control.recv().expect("Tx Gen control signal");
                        self.handle_signal(signal); 
                    },
                    State::Step(num_tx) => {
                        let transactions = self.generate_trans(num_tx);
                        self.send_to_mempool(transactions);

                        self.state = State::Pause;
                    },
                    State::Continuous(throttle) => {
                        // create transaction according to some distribution
                        match self.control.try_recv() {
                            Ok(signal) => {
                                self.handle_signal(signal);
                            },
                            Err(TryRecvError::Empty) => {
                                
                            },
                            Err(TryRecvError::Disconnected) => panic!("disconnected tx_gen control signal"),
                        }
                    },
                    State::Simulate => {
                        let transactions= self.generate_transaction_from_history();
                        self.estimate_gas(transactions);
                        self.state = State::Pause;
                        continue;
                    }
                }
                if let State::Continuous(throttle) = self.state {
                    if self.mempool.lock().unwrap().len() as u64 >= throttle {
                        // if the mempool is full, just skip this transaction
                        let interval: u64 = match &self.arrival_distribution {
                            ArrivalDistribution::Uniform(d) => d.interval,
                        };
                        let interval = time::Duration::from_micros(interval);
                        thread::sleep(interval);
                        continue;
                    }
                }

                let transaction = self.generate_trans(1000);
                let now = SystemTime::now();
                self.send_to_mempool(transaction);

                // sleep interbal
                let interval: u64 = match &self.arrival_distribution {
                    ArrivalDistribution::Uniform(d) => d.interval,
                };
                let interval = time::Duration::from_micros(interval);
                let time_spent = time::Instant::now().duration_since(tx_gen_start);
                let interval = {
                    if interval > time_spent {
                        interval - time_spent
                    } else {
                        time::Duration::new(0, 0)
                    }
                };
                thread::sleep(interval);

            }
        });
    }

    fn estimate_gas(&mut self, transactions: Vec<Transaction>) {
        let mut mempool = self.mempool.lock().expect("tx gen lock mempool");
        for tx in transactions {
            mempool.estimate_gas(tx);
        }
        drop(mempool);
    }

    fn send_to_mempool(&mut self, transactions: Vec<Transaction>) {
        let mut mempool = self.mempool.lock().expect("tx gen lock mempool");
        mempool.insert_transactions(transactions);
        drop(mempool);
    }


    pub fn handle_signal(&mut self, signal: TxGenSignal) {
        match signal {
            TxGenSignal::Start(t) => {
                self.state = State::Continuous(t);
            },
            TxGenSignal::Stop => {
                self.state = State::Pause;
            },
            TxGenSignal::Step(num) => {
                self.state = State::Step(num);
            },
            TxGenSignal::Simulate => {
                self.state = State::Simulate;
                println!("Start simulation");
            }
        }
    }

    pub fn generate_trans(&self, num: usize) -> Vec<Transaction>  {
        let mut transactions: Vec<Transaction> = vec![];
        let mut rng = rand::thread_rng();
        //let now = SystemTime::now();
        for _ in 0..num {
            let bytes_size = 128;
            let input = TransactionInput {
                previous_output: OutPoint::default(),
                script_sig: Bytes::new_with_len(bytes_size), //magic
                sequence: 0,
                script_witness: vec![],
            };
            let output = TransactionOutput {
                value: rng.gen(),
                script_pubkey: Bytes::new_with_len(bytes_size),
            };
            let tx = Transaction {
                version: 0,
                inputs: vec![input],
                outputs: vec![output] ,
                lock_time: 0, 
            };
            transactions.push(tx); 
        }
        //info!("elapsed {:?}", now.elapsed().unwrap());
        PERFORMANCE_COUNTER.record_generated_transactions(num);
        transactions
    }

    fn generate_transaction_from_history(&self) -> Vec<Transaction> {
        // please change

        //let mut file = File::create("gas_history.csv").unwrap();
        let mut transactions: Vec<Transaction> = vec![];
        let request_url = format!("http://api.etherscan.io/api?module=account&action=txlist&address={address}&startblock={start}&endblock={end}&sort=asc&apikey={apikey}&page={page}&offset={offset}",
                                  address = "0x06012c8cf97bead5deae237070f9587f8e7a266d",//"0x732de7495deecae6424c3fc3c46e47d6b4c5374e",
                                  start = 5752558,
                                  end = 9463322,
                                  apikey = "UGEFW13C4HVZ9GGH5GWIRHQHYYPYKX7FCX",
                                  page = 1,
                                  offset = 1000);
        ////let request_url = format!("http://api.etherscan.io/api?module=account&action=txlist&address={address}&startblock={start}&endblock={end}&sort=asc&apikey={apikey}&page={page}&offset={offset}",
                                  ////address = "0x1985365e9f78359a9B6AD760e32412f4a445E862",
                                  ////start = 8752558,
                                  ////end = 9478235,
                                  ////apikey = "UGEFW13C4HVZ9GGH5GWIRHQHYYPYKX7FCX",
                                  ////page = 1,
                                  ////offset = 1000);
        //println!("{:?}", request_url);
        //let response = requests::get(request_url).unwrap();
        //let data = response.json().unwrap();
        //let data = reqwest::blocking::get(&request_url).unwrap()
           //.json::<HashMap<String, String>>().unwrap(); 
        
        
        //let txs = data["result"].clone();
        //let mut i = 0;
        //for tx in txs.members() {
            //let isError = tx["isError"].as_str().unwrap().parse::<i32>().unwrap();

            //if isError == 0 && tx["to"].as_str().unwrap() == "0x06012c8cf97bead5deae237070f9587f8e7a266d" {
                //////if isError == 0 && tx["to"].as_str().unwrap() == "0x1985365e9f78359a9b6ad760e32412f4a445e862" {


                //let mut transaction = Transaction::default();
                //let content = String::from(tx["input"].as_str().unwrap()).replace("0x", "");
                //let mut txinput = TransactionInput::coinbase(Bytes::from(hex::decode(content.as_str()).expect("decode error")));
                //transaction.inputs.push(txinput);
                ////let tx_hash = String::from(tx["hash"].as_str().unwrap());
                ////let content = String::from(tx["input"].as_str().unwrap()).replace("0x", "");
                ////let address = String::from(tx["from"].as_str().unwrap());
                ////let gas_used = String::from(tx["gas"].as_str().unwrap());
                ////let gas_used = usize::from_str_radix(&gas_used, 10).unwrap();
                //i += 1;
                ////file.write_all(format!("{},{}\n",i,gas_used).as_bytes());
                ////let mut tx = Transaction{
                    ////inputs: vec![Input {
                        ////tx_hash: H256::from(tx_hash),
                        ////index: 0,
                        ////unlock: H256::new(),
                        ////content: hex::decode(&content).unwrap()
                    ////}],
                    ////outputs: vec![Output {
                        ////address: self.my_addr[0],
                        ////value: 10
                    ////}],
                    ////is_coinbase: false,
                    ////hash: H256::default()
                ////};
                ////tx.update_hash();
                //transactions.push(transaction);
            //}
        //}
        //println!("generate {} txs from history", transactions.len());
        return transactions;

    }
}
