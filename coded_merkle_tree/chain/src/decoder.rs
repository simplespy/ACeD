use constants::{BASE_SYMBOL_SIZE, AGGREGATE, RATE, HEADER_SIZE, TRANSACTION_SIZE};
use std::cmp;
use std::ops::BitXor;
use {Symbols, SymbolBase, SymbolUp};
use hash::H256;
use crypto::dhash256;
use rand::distributions::{Distribution, Bernoulli, Uniform};
use serde::{Serialize,Deserialize};
use big_array::BigArray;
use std::time::SystemTime;
use std::collections::HashSet;
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use bytes::Bytes;
use {BlockHeader, Transaction};
use ser::{deserialize, serialize};

// Symbols on the base layer can have different size as the upper layer
// The value of symbol is empty before it is decoded
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Symbol {
    #[serde(with = "BigArray")]
	Base([u8; BASE_SYMBOL_SIZE]),
    #[serde(with = "BigArray")]
	Upper([u8; 32 * AGGREGATE]),
	Empty,
}

impl Symbol{
    pub fn bitxor(&mut self, y: &[u8]) {
        if let Symbol::Base(ref mut x) = *self {
            for j in 0..BASE_SYMBOL_SIZE {
                x[j] = x[j].bitxor(y[j]);
            }
        } else {
            if let Symbol::Upper(ref mut x) = *self {
                for j in 0..(32 * AGGREGATE) {
                    x[j] = x[j].bitxor(y[j]);
                }
            }
        }
    }

}

impl std::fmt::Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,"symbol")
    }
}

// a new type indicating types of coding errors
// NotZero: symbols in a parity equation does not sum up to zero
// NotHash: decoded symbol does not match its hash
// Stopped: peeling decoder cannot continue due to absence of degree-one parity node 
pub enum CodingErr{
	NotZero,
	NotHash,
	Stopped,
} 

// a full node sends an incorrect-coding proof if it detects errors during decoding
pub struct IncorrectCodingProof {
	pub error_type: CodingErr,
	pub level: u32,
	pub symbols: Vec<Symbol>,
	pub indices: Vec<u64>,
	pub parity_index: u64,
	pub proofs: Vec<Vec<Symbol>>,
	pub stop_set: Vec<u64>,
	pub stop_ratio: f32
}

//A code is specified by its parity-check matrix, which is represented by parities or symbols vectors
#[derive(Clone)]
pub struct Code {
	pub parities: Vec<Vec<u64>>,
	pub symbols: Vec<Vec<u64>>,
}

// Decoder for CMT
// contains a decoder for each layer of CMT
// hashes are hashes of the coded symbols on the last (top) layer
pub struct TreeDecoder {
	pub n: u64, //block length of code on the base layer of the tree
	pub height: u32,
	pub decoders: Vec<Decoder>,
	pub hashes: Vec<Vec<H256>> //hashes of all layers
}

#[derive(Clone)]
pub struct Decoder {
	pub level: u32, // layer index of CMT
	pub n: u64, // # of coded symbols
	pub k: u64, // # of systematic symbols
	pub p: u64, // # of parity check equations

	pub code: Code, //code shall not change during decoding

	pub parities: Vec<Vec<u64>>, // vector of length p, each element is a vector indicating the variable nodes connected to a parity node
	pub symbols: Vec<Vec<u64>>, // vector of length n, each element is a vector indicating the parity nodes connected to a variable node

    pub parities_set: Vec<HashSet<u64>>,

	pub symbol_values: Vec<Symbol>, // values of variable nodes
    pub parity_values: Vec<Symbol>, //values of parity nodes
    pub parity_degree: Vec<u32>, 
    pub degree_1_parities: Vec<u64>, // set of parity nodes whose degree is 1 during decoding

    pub num_decoded_sys_symbols: u64,
    pub num_decoded_symbols: u64,
}

//Convert decoded symbols of the current layer to the hashes of the previous layer
fn symbol_to_hash(symbols: &Vec<Symbol>) -> Vec<H256> {
    let reduce_factor = ((
            AGGREGATE as f32) * RATE) as u64;

	let number_of_hashes = symbols.len() * AGGREGATE; 
	let mut previous_hashes = vec![H256::default();number_of_hashes];

    //convert each symbol to a vector of hashes
	let mut symbols_in_hashes: Vec<SymbolUp> = vec![];
	for i in 0..symbols.len() {
		//convert symbols[i] to a vector of hashes
		let mut symbol_in_hash = [H256::default(); AGGREGATE];
		if let Symbol::Upper(symbol_in_bytes) = symbols[i] {
			for j in 0..AGGREGATE {
				let mut h = [0u8; 32];
				h.copy_from_slice(&symbol_in_bytes[j*32..(j*32+32)]);
			    symbol_in_hash[j] = H256::from(h);
            }
		}
		symbols_in_hashes.push(symbol_in_hash);
	}
    
    //number of systematic symbols on the previous level
	let k = ((previous_hashes.len() as f32) * RATE) as u64;

	for index in 0..previous_hashes.len() {
		let mut hash_index = 0;

        // current symbol is a systematic symbol
		if (index as u64) <= k - 1 {
			hash_index = (index as u64) % reduce_factor;
		}
		// current symbol is a parity symbol
		else {
			hash_index = ((index as u64) - k) % ((AGGREGATE as u64) - reduce_factor) + reduce_factor;
		}

		previous_hashes[index] = symbols_in_hashes[next_index(index as u64, k, reduce_factor) as usize][hash_index as usize];
	}

	previous_hashes
}

//return if a symbol is equal to zero or not (every byte equals to 0u8)
fn symbol_equal_to_zero(symbol: Symbol) -> bool {
	let mut flag = true;
	match symbol {
		Symbol::Base(decoded) => {
			for j in 0..BASE_SYMBOL_SIZE {
				if decoded[j] != 0u8 {
					flag = false;
					break;
				}
			}	
		},
		Symbol::Upper(decoded) => {
			for j in 0..32 * AGGREGATE {
				if decoded[j] != 0u8 {
					flag = false;
					break;
				}
			}	
		},
		_ => {}
	}
	flag
}

//index of the parent symbol on the coded Merkle tree
fn next_index(index: u64, k: u64, reduce_factor: u64) -> u64 {
	if index <= k - 1 {
		index / reduce_factor
	}
	else {
		(index - k) / ((AGGREGATE as u64) - reduce_factor)
	}
}

fn remove_one_item_local(vector: &mut Vec<u64>, item: &u64) {
    for i in (0..vector.len()).rev() {
        if vector[i] == *item {
            vector.swap_remove(i);
            break;
        }
    }
}

fn remove_one_item(vector: &Vec<u64>, item: &u64) -> Vec<u64> {
	let mut new_vec = vec![]; 
	for i in 0..vector.len() {
		if vector[i] != *item {
			new_vec.push(vector[i].clone());
		}
	}
	new_vec
}

//IncorrectCodingProof
pub fn check_incorrect_coding(i: usize, decoder: &mut Decoder) -> Result<(), (usize, u64, Vec<Symbol>, Vec<u64>) > {
    for j in 0..decoder.p {
        if decoder.parity_degree[j as usize] == 0 { //all symbols associated to this parity are known
            if !symbol_equal_to_zero(decoder.parity_values[j as usize]) {
                //construct incorrect coding proof
                let error_indices = decoder.code.parities[j as usize].clone();
                let mut error_symbols: Vec<Symbol> = vec![];
    
                for t in error_indices.iter() {
                    error_symbols.push(decoder.symbol_values[*t as usize]);
                }
                println!("NotZero incorrect coding detected on layer {} for parity equation #{}.",i,j);
                return Err((i,j as u64, error_symbols, error_indices));
            } 
        }
    }
    Ok(())
}

impl TreeDecoder {

    //pub fn layer_decoder(
        //&mut self, 
        //i: u32, 
        //symbols_all_levels: &Vec<Vec<Symbol>>, 
        //indices_all_levels: &Vec<Vec<u64>>
    //) -> Result<(), IncorrectCodingProof> {
        //let mut decoder = &mut self.decoders[i as usize];
        //let received_symbols = &symbols_all_levels[i as usize];
        //let received_indices = &indices_all_levels[i as usize];
        //let (mut new_symbols, 
             //mut new_symbol_indices, 
             //mut decoded) = decoder.symbol_update_from_reception(received_symbols, received_indices);

        //let mut progress = decoder.parity_update(new_symbols, new_symbol_indices);

        //match check_incorrect_coding(i, decoder) {
            //Ok(()) => (),
            //Err((i, j, error_symbols, error_indices)) => {
                //let incorrect_proof = self.generate_incorrect_coding_proof(
                        //CodingErr::NotZero, i, j as u64, error_symbols, error_indices, vec![], 1.0);
                //return Err(incorrect_proof);
            //}
        //}

        //if decoded {
            //if i > 0 {
                ////decoding done for layer i, use the systematic symbols as the hash proof for previous layer
                //self.hashes[(i-1) as usize] = symbol_to_hash(
                    //&decoder.symbol_values[0..(decoder.k as usize)].to_vec()
                //);
                ////hash_proof = self.hashes[(i-1) as usize].clone();
                //return Ok(());	
            //} else {
                //println!("Coded Merkle tree successfully decoded.");
                //return Ok(()); //Entire coded Merkle tree is decoded
            //}							
        //}
        
    //}

    pub fn init_parity_update(
        mut self, 
        i: usize,
        symbols_all_levels: &Vec<Vec<Symbol>>, 
        indices_all_levels: &Vec<Vec<u64>>
    ) -> Result<(), IncorrectCodingProof> {
        let mut decoder = &mut self.decoders[i];
        let received_symbols = &symbols_all_levels[i];
        let received_indices = &indices_all_levels[i];
        let (mut new_symbols, 
             mut new_symbol_indices, 
             mut decoded) = decoder.symbol_update_from_reception(received_symbols, received_indices);

        let mut progress = decoder.parity_update(new_symbols, new_symbol_indices);

        match check_incorrect_coding(i, decoder) {
            Ok(()) => (),
            Err((i, j, error_symbols, error_indices)) => {
                let incorrect_proof = self.generate_incorrect_coding_proof(
                        CodingErr::NotZero, i as u32, j as u64, error_symbols, error_indices, vec![], 1.0);
                return Err(incorrect_proof);
            }
        }

        if decoded {
            if i > 0 {
                //decoding done for layer i, use the systematic symbols as the hash proof for previous layer
                self.hashes[(i-1) as usize] = symbol_to_hash(
                    &decoder.symbol_values[0..(decoder.k as usize)].to_vec()
                );
                //hash_proof = self.hashes[(i-1) as usize].clone();
                return Ok(());	
            } else {
                println!("Coded Merkle tree successfully decoded.");
                return Ok(()); //Entire coded Merkle tree is decoded
            }							
        }

        Ok(())
    }

    //pub fn loop_parity_update(&mut self, i: usize, progress: bool) -> Result<Vec<Vec<H256>>, IncorrectCodingProof> {
        //let mut decoder = &mut self.decoders[i];
        //let mut hash_proof = Vec<Vec<H256>>;
        //loop {
            ////check for degree-1 parity nodes, if no such nodes are found, decoding is stalled
            //if progress {
                //let mut decoding_result = decoder.symbol_update_from_degree_1_parities(&hash_proof);
                //match decoding_result {
                    //Ok((dec_syms, dec_sym_indices, finished)) => { //all decoded symbols match their hash 
                        ////Update the parity values
                        //progress = decoder.parity_update(dec_syms, dec_sym_indices);
                        //match check_incorrect_coding(i as usize, decoder) {
                            //Ok(()) => (),
                            //Err((i, j, error_symbols, error_indices)) => {
                                //let incorrect_proof = self.generate_incorrect_coding_proof(
                                        //CodingErr::NotZero, 
                                        //i as u32, 
                                        //j as u64, 
                                        //error_symbols, 
                                        //error_indices, 
                                        //vec![], 
                                        //1.0);
                                //return Err(incorrect_proof);
                            //}
                        //}
                        //if finished { //decoding is correctly done for layer i 
                            //if i > 0 { //not the base layer yet
                                ////use the systematic symbols as the hash proof for previous layer
                                //self.hashes[(i-1) as usize] = symbol_to_hash(&decoder.symbol_values[0..(decoder.k as usize)].to_vec());
                                //hash_proof = self.hashes[(i-1) as usize].clone();
                                //decoded = finished;
                                //return hash_proof;
                            //} else { //base layer decoded
                                //// modify
                                //println!("Coded Merkle tree successfully decoded.");
                                //return Ok(());
                            //} 				                
                        //} else { //decoding for layer i needs to continue 
                            //continue;
                        //}
                    //},
                    ////some decoded symbols do not pass hash test
                    //Err((err_level, err_parity, index_set, proof_symbols)) => { 
                        //return Err(self.generate_incorrect_coding_proof(CodingErr::NotHash, err_level, 
                                    //err_parity, proof_symbols, index_set, vec![], 1.0));
                    //}
                //} 
            //} else {
                ////no more progress can be made, a stopping set is found
                ////construct a Stopped incorrect-coding proof using the indices of the encountered stopping set
                //let mut stopping_set = vec![];
                //for sym_idx in 0..self.decoders[i as usize].n {
                    //if let Symbol::Empty = self.decoders[i as usize].symbol_values[sym_idx as usize] {
                        //stopping_set.push(sym_idx.clone());
                    //}
                //}
                //let stopping_ratio = (stopping_set.len() as f32) / (self.decoders[i as usize].n as f32);

                //println!("Hitting a stopping set at layer {}. Decoding failed with a stopping ratio of {}.", i, stopping_ratio);
                ////panic!("Hitting a stopping set at layer {}. Decoding failured.", i);
                //return Err(self.generate_incorrect_coding_proof(CodingErr::Stopped, i as u32, 
                        //0u64, vec![], vec![], stopping_set, stopping_ratio));
            //}
        //}

    //} 


	//Decode coded Merkle tree after receiving enough symbols on each level
	pub fn run_tree_decoder(
        &mut self, 
        symbols_all_levels: Vec<Vec<Symbol>>, 
        indices_all_levels: Vec<Vec<u64>>, 
        header: BlockHeader,
    ) -> Result<(Vec<Transaction>), IncorrectCodingProof> {
		//hashes of the symbols being decoded. For top layer, they are stored in the header
		let mut hash_proof = self.hashes[(self.height - 1) as usize].clone();

        let mut trans_bytes: Vec<u8> = vec![];

		//Iterate decoding starting from the top level of coded Merkle tree
		for i in (0..self.height as usize).rev() {
			let received_symbols = symbols_all_levels[i as usize].clone();
			let received_indices = indices_all_levels[i as usize].clone();
			//Here the variable decoded is used for indicating layer i gets decoded
            let mut decoder = &mut self.decoders[i as usize];
			let (mut new_symbols, 
                 mut new_symbol_indices, 
                 mut decoded) = decoder.symbol_update_from_reception(&received_symbols, &received_indices);

			//Update the parities using the received symbols
			let mut progress = decoder.parity_update(new_symbols, new_symbol_indices);

			//parity nodes are updated, now check if there is any incorrect coding
            match check_incorrect_coding(i, decoder) {
                Ok(()) => (),
                Err((i, j, error_symbols, error_indices)) => {
                    let incorrect_proof = self.generate_incorrect_coding_proof(
                            CodingErr::NotZero, 
                            i as u32, 
                            j as u64, 
                            error_symbols, 
                            error_indices, 
                            vec![], 
                            1.0);
                    return Err(incorrect_proof);
                }
            }

            //Already received all coded symbols and all parity equations are satisfied
			if decoded {
				if i > 0 {
					//decoding done for layer i, use the systematic symbols as the hash proof for previous layer
				    self.hashes[(i-1) as usize] = symbol_to_hash(&decoder.symbol_values[0..(decoder.k as usize)].to_vec());
				    hash_proof = self.hashes[(i-1) as usize].clone();
				    continue;	
				} else {
                    let base_decoder = &self.decoders[i];
                    let symbols = base_decoder.symbol_values[0..base_decoder.k as usize].to_vec();

                    let mut bytes: Vec<u8> = vec![];                   
                    for symbol in symbols.clone() {
                        if let Symbol::Base(s) = symbol {
                            bytes.extend_from_slice(&s[0..BASE_SYMBOL_SIZE]);
                        }
                    }

                    let mut transactions_byte: Vec<Bytes> = vec![];                   
                    let mut transactions: Vec<Transaction> = vec![];
                    let num_trans = bytes.len() / TRANSACTION_SIZE as usize; //TODO magic
                    for d in 0..(num_trans-1) {
                        let l = d*TRANSACTION_SIZE as usize;
                        let u = (d+1)*TRANSACTION_SIZE as usize;
                        let b: Bytes = bytes[l..u].into();
                        let t: Transaction = deserialize(&b as &[u8]).unwrap();
                        transactions_byte.push(b);
                        transactions.push(t);
                    }
                    println!("Coded Merkle tree successfully decoded {}.", transactions_byte.len());
                    return Ok(transactions);
				}							
			}

			//Start decoding layer i using degree-1 parities, until all symbols are decoded or hitting a stopping
			loop {
				//check for degree-1 parity nodes, if no such nodes are found, decoding is stalled
				if progress {
					let mut decoding_result = decoder.symbol_update_from_degree_1_parities(&hash_proof);
					match decoding_result {
						Ok((dec_syms, dec_sym_indices, finished)) => { //all decoded symbols match their hash 
							//Update the parity values
							progress = decoder.parity_update(dec_syms, dec_sym_indices);
                            match check_incorrect_coding(i as usize, decoder) {
                                Ok(()) => (),
                                Err((i, j, error_symbols, error_indices)) => {
                                    let incorrect_proof = self.generate_incorrect_coding_proof(
                                            CodingErr::NotZero, 
                                            i as u32, 
                                            j as u64, 
                                            error_symbols, 
                                            error_indices, 
                                            vec![], 
                                            1.0);
                                    return Err(incorrect_proof);
                                }
                            }
			                if finished { //decoding is correctly done for layer i 
			                	if i > 0 { //not the base layer yet
					            //decoding done for layer i, use the systematic symbols as the hash proof for previous layer
				                    self.hashes[(i-1) as usize] = symbol_to_hash(&decoder.symbol_values[0..(decoder.k as usize)].to_vec());
				                    hash_proof = self.hashes[(i-1) as usize].clone();
				                    decoded = finished;
				                    break;
				                } else { //base layer decoded
                                    let base_decoder = &self.decoders[i];
                                    let symbols = base_decoder.symbol_values[0..base_decoder.k as usize].to_vec();

                                    let mut bytes: Vec<u8> = vec![];                   
                                    for symbol in symbols.clone() {
                                        if let Symbol::Base(s) = symbol {
                                            bytes.extend_from_slice(&s[0..BASE_SYMBOL_SIZE]);
                                        }
                                    }

                                    let mut transactions_byte: Vec<Bytes> = vec![];                   
                                    let mut transactions: Vec<Transaction> = vec![];
                                    let num_trans = bytes.len() / TRANSACTION_SIZE as usize; //TODO magic
                                    for d in 0..(num_trans-1) {
                                        let l = d*TRANSACTION_SIZE as usize;
                                        let u = (d+1)*TRANSACTION_SIZE as usize;
                                        let b: Bytes = bytes[l..u].into();
                                        let t: Transaction = deserialize(&b as &[u8]).unwrap();
                                        transactions_byte.push(b);
                                        transactions.push(t);
                                    }

                                    println!("Coded Merkle tree successfully decoded {}.", transactions_byte.len());
                                    return Ok(transactions);                   
				                } 				                
				            } else { //decoding for layer i needs to continue 
				            	continue;
				            }
						},
                        //some decoded symbols do not pass hash test
						Err((err_level, err_parity, index_set, proof_symbols)) => { 
							return Err(self.generate_incorrect_coding_proof(CodingErr::NotHash, err_level, 
						            	err_parity, proof_symbols, index_set, vec![], 1.0));
						}
					} 
				} else {
					//no more progress can be made, a stopping set is found
					//construct a Stopped incorrect-coding proof using the indices of the encountered stopping set
					let mut stopping_set = vec![];
					for sym_idx in 0..self.decoders[i as usize].n {
						if let Symbol::Empty = self.decoders[i as usize].symbol_values[sym_idx as usize] {
							stopping_set.push(sym_idx.clone());
						}
					}
					let stopping_ratio = (stopping_set.len() as f32) / (self.decoders[i as usize].n as f32);

					println!("Hitting a stopping set at layer {}. Decoding failed with a stopping ratio of {}.", i, stopping_ratio);
					//panic!("Hitting a stopping set at layer {}. Decoding failured.", i);
					return Err(self.generate_incorrect_coding_proof(CodingErr::Stopped, i as u32, 
						    0u64, vec![], vec![], stopping_set, stopping_ratio));
				}
			}

			if decoded {
				if i > 0 {
					continue;
				} else {
                    let base_decoder = &self.decoders[i];
                    let symbols = base_decoder.symbol_values[0..base_decoder.k as usize].to_vec();

                    let mut bytes: Vec<u8> = vec![];                   
                    for symbol in symbols.clone() {
                        if let Symbol::Base(s) = symbol {
                            bytes.extend_from_slice(&s[0..BASE_SYMBOL_SIZE]);
                        }
                    }

                    let mut transactions_byte: Vec<Bytes> = vec![];                   
                    let mut transactions: Vec<Transaction> = vec![];
                    let num_trans = bytes.len() / TRANSACTION_SIZE as usize; //TODO magic
                    for d in 0..(num_trans-1) {
                        let l = d*TRANSACTION_SIZE as usize;
                        let u = (d+1)*TRANSACTION_SIZE as usize;
                        let b: Bytes = bytes[l..u].into();
                        let t: Transaction = deserialize(&b as &[u8]).unwrap();
                        transactions_byte.push(b);
                        transactions.push(t);
                    }

                    println!("Coded Merkle tree successfully decoded {}.", transactions_byte.len());
                    return Ok(transactions);                   
				}
			} 
		}
		unreachable!();
		//Ok(self.decoders.clone())
	}

    //Initialize the tree decoder
	pub fn new(codes: Vec<Code>, header_hash: &Vec<H256>) -> Self {
		let num_layers = codes.len();
		let base_length: u64 = codes[0].symbols.len() as u64;
		let mut decs: Vec<Decoder> = vec![];
		let mut hash_list: Vec<Vec<H256>> = vec![];
		for i in 0..num_layers {
			let code = &codes[i];
			let dec: Decoder = Decoder::new(i as u32, code.parities.to_vec(), code.symbols.to_vec());
			decs.push(dec);
			hash_list.push(vec![H256::default();code.symbols.len()]);
		}
		hash_list[num_layers-1] = header_hash.to_vec();

		TreeDecoder {
			n: base_length,
			height: num_layers as u32,
			decoders: decs,
			hashes: hash_list,
		}
	}

	//Generate merkle proof for a symbol  
	pub fn generate_merkle_proof(&self, lvl: usize, index: u64) -> Vec<Symbol> {
		let header_size = self.hashes.len();

		let mut proof = Vec::<Symbol>::new();
		let mut moving_index = index;
		let mut moving_k = self.decoders[lvl].k;
		let reduce_factor = ((AGGREGATE as f32) * RATE) as u64;
		for i in lvl..((self.height - 1) as usize) {
			moving_index = next_index(moving_index, moving_k, reduce_factor);
            proof.push(self.decoders[i+1].symbol_values[moving_index as usize].clone());
            moving_k = moving_k / reduce_factor;
		}
		proof
	}

	pub fn generate_incorrect_coding_proof(&self, err_type: CodingErr, lvl: u32, parity: u64, 
		symbols: Vec<Symbol>, indices: Vec<u64>, stopping_set: Vec<u64>, stopping_ratio: f32) -> IncorrectCodingProof {
		let mut merkle_proofs: Vec<Vec<Symbol>> = vec![];
		for i in 0..indices.len() {
			merkle_proofs.push(self.generate_merkle_proof(lvl as usize, indices[i]));
		}
		IncorrectCodingProof {
			error_type: err_type,
	        level: lvl,
	        symbols: symbols,
	        indices: indices,
	        parity_index: parity,
	        proofs: merkle_proofs,
	        stop_set: stopping_set,
	        stop_ratio: stopping_ratio
	    }
	}
}

#[derive(Clone, Debug)]
pub enum Message {
    Control(Sender<(usize, Vec<Symbol>)>),
    Data(Symbol, usize),
}


impl Decoder {
	// Initialize the decoder for a layer of CMT 
	pub fn new(level: u32, parities: Vec<Vec<u64>>, symbols: Vec<Vec<u64>>) -> Self {
		let n: u64 = symbols.len() as u64; //number of coded symbols
		let p: u64 = parities.len() as u64; //number of parity nodes
		let k: u64 = ((n as f32) * RATE) as u64; //number of systematic symbols

        let mut parities_set: Vec<HashSet<u64>> = vec![];
        for parity in parities.iter() {
            parities_set.push(parity.clone().into_iter().collect());
        }

		let mut parity_deg = vec![0u32; p as usize]; //number of variable nodes a parity node is connected to, this changes during peeling decoding
		for i in 0..(p as usize) {
			parity_deg[i] = parities[i].len() as u32; 
		}

		let mut parity_val = Vec::<Symbol>::new(); //values of parity nodes
		let mut symbol_val = Vec::<Symbol>::new(); //values of variable nodes

		match level {
			0 => {
				for _ in 0..p {
					parity_val.push(Symbol::Base([0u8; BASE_SYMBOL_SIZE]));
				}
			},
			_ => {
				for _ in 0..p {
					parity_val.push(Symbol::Upper([0u8; 32 * AGGREGATE]));
				}
			},
		}

		for _ in 0..n {
			symbol_val.push(Symbol::Empty);
		}

		Decoder {
			level: level, n: n, k: k, p: p,
			code: Code {parities: parities.clone(), symbols: symbols.clone()},
			parities: parities, symbols: symbols,
            parities_set:  parities_set,
			symbol_values: symbol_val,
			parity_values: parity_val,
			parity_degree: parity_deg,
			degree_1_parities: vec![],
			num_decoded_sys_symbols: 0, num_decoded_symbols: 0,
		}
	}

    //decode new symbols simply from receiving them
    // out symbols are the new systematic symbols 
	pub fn symbol_update_from_reception(&mut self, symbols: &Vec<Symbol>, symbol_indices: &Vec<u64>) -> (Vec<Symbol>,
		Vec<u64>, bool) {
		let mut out_symbols = Vec::<Symbol>::new();
        let mut out_indices = Vec::<u64>::new();

        let length = cmp::min(symbols.len(), symbol_indices.len());
        for i in 0..length {
        	if let Symbol::Empty = self.symbol_values[(symbol_indices[i] as usize)] {
        		self.symbol_values[(symbol_indices[i] as usize)] = symbols[i].clone();
        		self.num_decoded_symbols += 1;
        		if symbol_indices[i] < self.k {
        			self.num_decoded_sys_symbols += 1;
        		}
        		//output the updated symbols for future peeling decoding
        		//the output is a subset of input
        		out_symbols.push(symbols[i].clone());
        	    out_indices.push(symbol_indices[i].clone());
        	}        
        }

        (out_symbols, out_indices, self.num_decoded_symbols == self.n)
	}

    pub fn start_parallel_engine(&mut self, num_thread: usize) -> Vec<Sender<Message>> {
        let mut senders = vec![];
        let mut joins = vec![];
        for i in 0..num_thread {
            let (tx, rx) =channel();
            senders.push(tx);

            let mut parity_values = self.parity_values.clone();
            joins.push(thread::spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(message) => match message {
                            Message::Data(symbol, parity) => {
                                match symbol {
                                    Symbol::Base(x) => parity_values[parity as usize].bitxor(&x),
                                    Symbol::Upper(x) => parity_values[parity as usize].bitxor(&x),
                                    _ => {},
                                }
                            },
                            Message::Control(result_tx) => {
                                // return number
                                result_tx.send((i as usize, parity_values));
                                break;
                            }
                        },
                        Err(e) => println!("err {:?}",e),
                    }
                }
            }));
        }
        senders
    }

    //Update the values of parity nodes using decoded/received symbols
	pub fn parity_update(&mut self, symbols: Vec<Symbol>, symbol_indices: Vec<u64>) -> bool {
		if  symbols.len() == 0 {
			return self.degree_1_parities.len() != 0;
		}
		let length = cmp::min(symbols.len(), symbol_indices.len());
        let mut d = 0;
        let num_thread: usize = 5;
        let mut senders = self.start_parallel_engine(num_thread);
        let start = SystemTime::now();
		for i in 0..length {
			let (s, idx) = (&symbols[i], symbol_indices[i].clone());
			let parity_list = &self.symbols[idx as usize]; // subset of parity nodes that will be affected by symbol s
			for parity in parity_list.iter() {
				//Update the value of each parity node symbol s connects to
                senders[(*parity%(num_thread as u64)) as usize].send(Message::Data(*s, *parity as usize));

				self.parity_degree[*parity as usize] -= 1;
				if self.parity_degree[*parity as usize] == 1 {
                    self.degree_1_parities.push(parity.clone());
				}
                self.parities_set[*parity as usize].remove(&idx);
                d +=1;
			}
		}

        // collect results
        let mut receivers = vec![];
        for i in 0..num_thread {
            let (tx, rx) = channel(); 
            senders[i as usize].send(Message::Control(tx));
            receivers.push(rx);
        }

        let mut num = 0;
        let p = self.parity_values.len();
        let u = (f64::from(p as i32)/f64::from(num_thread as i32)).ceil() as usize;
        loop {
            for i in 0..num_thread { 
                match receivers[i as usize].try_recv() {
                    Ok((j, parities)) => {
                        num += 1;
                        for k in 0..u {
                            let idx: usize = j + k*num_thread ;
                            if idx < p {
                                self.parity_values[idx] = parities[idx];
                            }
                        }
                    },
                    _ => (),
                }
            }
            if num == num_thread {
                break;
            }
        }
        
        //println!("degree_1_parities {} d {} {:?}", self.degree_1_parities.len(), d, start.elapsed());
		self.degree_1_parities.len() != 0
	}

    pub fn parity_update_thread(&mut self, symbols: Vec<Symbol>, symbol_indices: Vec<u64>) -> bool {
		if  symbols.len() == 0 {
			return self.degree_1_parities.len() != 0;
		}
		let length = cmp::min(symbols.len(), symbol_indices.len());
        
        let mut d = 0;
        let start = SystemTime::now();
		for i in 0..length {
			let (s, idx) = (&symbols[i], symbol_indices[i].clone());
			let parity_list = &self.symbols[idx as usize]; // subset of parity nodes that will be affected by symbol s
			for parity in parity_list.iter() {
				//Update the value of each parity node symbol s connects to
                match s {
                    Symbol::Base(x) => self.parity_values[*parity as usize].bitxor(x),
                    Symbol::Upper(x) => self.parity_values[*parity as usize].bitxor(x),
                    _ => {},
                }

				self.parity_degree[*parity as usize] -= 1;
				if self.parity_degree[*parity as usize] == 1 {
                    self.degree_1_parities.push(parity.clone());
				}
                self.parities_set[*parity as usize].remove(&idx);
                
                d +=1;
				//self.parities[*parity as usize] = remove_one_item(&self.parities[*parity as usize], &idx);
				//self.symbols[idx as usize] = remove_one_item(&self.symbols[idx as usize], &parity);
			}
            //println!("d {}",d );
		}
		self.degree_1_parities.len() != 0
	}


	// pub fn symbol_update_from_degree_1_parities(&mut self) -> (Vec<Symbol>, Vec<u64>, bool) {
	// 	let mut symbols = Vec::<Symbol>::new();
 //        let mut symbol_indices = Vec::<u64>::new();

 //        for i in 0..self.degree_1_parities.len() {
 //        	let parity = self.degree_1_parities[i].clone();
 //        	if self.parities[parity as usize].len() > 0 {
 //        		let symbol_idx = self.parities[parity as usize][0];
 //        		if let Symbol::Empty = self.symbol_values[symbol_idx as usize] {
 //        			self.symbol_values[symbol_idx as usize] = self.parity_values[parity as usize];
 //        			self.num_decoded_symbols += 1; 
 //        			if symbol_idx < self.k {
 //                        self.num_decoded_sys_symbols += 1;
 //        			}
 //        			symbols.push(self.parity_values[parity as usize].clone());
 //                    symbol_indices.push(symbol_idx.clone());
 //        		} 
 //        	}
 //        }

 //        self.degree_1_parities = vec![];

 //        (symbols, symbol_indices, self.num_decoded_symbols == self.n)
	// }

    //Decode symbols using values of degree 1 parities. Decoding error may occur if the decoded symbol does not match its hash.
	pub fn symbol_update_from_degree_1_parities(&mut self, hashes: &Vec<H256>) 
	-> Result<(Vec<Symbol>, Vec<u64>, bool), (u32, u64, Vec<u64>, Vec<Symbol>)> {
		let mut symbols = Vec::<Symbol>::new();
        let mut symbol_indices = Vec::<u64>::new();

        for i in 0..self.degree_1_parities.len() {
        	let parity = self.degree_1_parities[i].clone();
        	if self.parities[parity as usize].len() > 0 {
        		let symbol_idx = self.parities[parity as usize][0];
        		// The only symbol connected to this parity node has not been decoded yet
        		if let Symbol::Empty = self.symbol_values[symbol_idx as usize] {
        			self.symbol_values[symbol_idx as usize] = self.parity_values[parity as usize]; //Symbol decoded

        			//now check if the decoded symbol matches its hash
        			let mut computed_hash = H256::default();
        			match self.symbol_values[symbol_idx as usize] {
        				Symbol::Base(decoded_sym) => {computed_hash = dhash256(&decoded_sym);},
        				Symbol::Upper(decoded_sym) => {computed_hash = dhash256(&decoded_sym);},
        				_ => {}
        			}
        			if computed_hash == hashes[symbol_idx as usize] {
        				self.num_decoded_symbols += 1; 
        			    if symbol_idx < self.k {
                            self.num_decoded_sys_symbols += 1;
                        }
        			    symbols.push(self.parity_values[parity as usize].clone());
                        symbol_indices.push(symbol_idx.clone());
                    } else {//coding is done incorrectly, return an incorrect-coding message
                    	println!("NotHash incorrect coding detected on layer {} for parity equation #{}.",self.level,parity);
                    	// Preparing info for constructing incorrect-coding proof
                    	let index_set: Vec<u64> = self.code.parities[parity as usize].clone();
                    	let mut correct_index_set: Vec<u64> = remove_one_item(&index_set, &symbol_idx);
                    	let mut symbols_in_proof: Vec<Symbol> = vec![];
                    	for j in 0..correct_index_set.len() {
                    		symbols_in_proof.push(self.symbol_values[j]);
                    	}
                    	correct_index_set.push(symbol_idx);
                    	return Err((self.level, parity, correct_index_set, symbols_in_proof));
                    }
                } 
            }
        }
        // value of the coded symbol connected to a degree 1 parity node is updated
        // remove this connection and set the degree_1_parities set to empty
        self.degree_1_parities = vec![];

        Ok((symbols, symbol_indices, self.num_decoded_symbols == self.n))
    }

   
    // this function encodes systematic symbols to obtain 
    // parity symbols, using the same decoding process
    // obtain new parity symbols through decoding from systematic symbols
	pub fn symbol_update_from_degree_1_parities_encode(&mut self) -> (Vec<Symbol>, Vec<u64>, bool) {
		let mut symbols = Vec::<Symbol>::new();
        let mut symbol_indices = Vec::<u64>::new();

        for parity in self.degree_1_parities.clone() {
        	if self.parities_set[parity as usize].len() > 0 {
        	//if self.parities[parity as usize].len() > 0 {
                //println!("greater than 0 {}", self.parities[parity as usize].len());
        		let symbol_idx = *(self.parities_set[parity as usize].iter().last().unwrap());
        		if let Symbol::Empty = self.symbol_values[symbol_idx as usize] {
        			self.symbol_values[symbol_idx as usize] = self.parity_values[parity as usize]; //Symbol decoded
        			self.num_decoded_symbols += 1; 
        			if symbol_idx < self.k {
                        self.num_decoded_sys_symbols += 1;
                    }
        			symbols.push(self.parity_values[parity as usize].clone());
                    symbol_indices.push(symbol_idx.clone());                    
                } 
            }
        }
        self.degree_1_parities = vec![];

        (symbols, symbol_indices, self.num_decoded_symbols == self.n)
    }

	pub fn peeling_encode(&mut self) -> bool {
		// iterative encoding/decoding until all parity symbols are found
		loop {
			let (symbols, symbol_indices, encoded) = self.symbol_update_from_degree_1_parities_encode();

			if encoded { return encoded; }
			if symbols.len() > 0 { // new symbols get decoded
				let keep_peeling = self.parity_update(symbols, symbol_indices); 
				//println!("The current parities vector is {:?}", self.parities);
				//println!("The set of degree-1 parity nodes are {:?}.", self.degree_1_parities);
				//println!("It is {} that more parity values are available.", keep_peeling);
				if keep_peeling {continue;} //new degree-1 parity created
			}
			// this part is unreachable during encoding
			println!("{} out of {} symbols are decoded.", self.num_decoded_symbols, self.n);
			return self.num_decoded_symbols == self.n;
		}
	}

	//Encoding by decoding all parity symbols from systematic symbols
	//The variable "correct" indicates if the encoding will be done correctly
	pub fn encode(&mut self, sys_symbols: Vec<Symbol>, correct: bool) -> Vec<Symbol> {
		let mut indices = vec![];//indices of sysmtematic symbols
		for i in 0..self.k {
			indices.push(i as u64);
		}
		let (symbols, symbol_indices, encoded) = self.symbol_update_from_reception(&sys_symbols, &indices);
        if !encoded {
            loop {
                // more degree 1 parity nodes are available, we can continue encoding
                if self.parity_update(symbols, symbol_indices) {
                    if self.peeling_encode() {
                        break;
                    } else {
                        unreachable!();
                    } 
                }
                else {
                    unreachable!(); 
                }
            }
        }

		let mut output_symbols = self.symbol_values.clone();
		if !correct { // flip the 1st parity symbol (kth symbol overall)
			if self.level == 0 { //This is base layer
			    let mut parity = [0u8; BASE_SYMBOL_SIZE];
			    if let Symbol::Base(sym) = self.symbol_values[self.k as usize] {
			    	for l in 0..BASE_SYMBOL_SIZE {
					    parity[l] = sym[l].bitxor(255u8);
					}
			    }
			    output_symbols[self.k as usize] = Symbol::Base(parity);
			} else { //This is higher layer
			    let mut parity_up = [0u8; 32 * AGGREGATE];
			    if let Symbol::Upper(sym_up) = self.symbol_values[self.k as usize] {
			    	for l in 0..(32 * AGGREGATE) {
					    parity_up[l] = sym_up[l].bitxor(255u8);
					}
			    }
			    output_symbols[self.k as usize] = Symbol::Upper(parity_up);
			}
		}
		output_symbols
	}
}

// #[cfg(test)]
// mod tests {
// 	use rand::thread_rng;
//     use rand::seq::SliceRandom;
// 	use super::*;

// 	//Test for decoder for a (2,4)-regular LDPC code with (n,k) = (8,4)
// 	#[test]
// 	fn test_peeling_decoder1() {
// 		// let mut vec: Vec<u64> = (0..16).collect();
//   //       vec.shuffle(&mut thread_rng());

//   //       let mut symbols: Vec<Vec<u64>> = vec![];
//   //       for i in 0..8 {
//   //       	symbols.push(vec![vec[2*i]/4, vec[2*i+1]/4]);
//   //       }

// 		// let mut parities: Vec<Vec<u64>> = vec![];
// 		// for i in 0..4 {
// 		// 	let mut parity = vec![];
// 		// 	for j in 0..8 {
// 		// 		if symbols[j].contains(&i) {
// 		// 			parity.push(j as u64);
// 		// 		}
// 		// 	}
// 		// 	parities.push(parity);
// 		// }

// 		let symbols: Vec<Vec<u64>> = vec![
// 		vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 0], vec![0, 3], vec![1, 2], vec![2, 1], vec![3, 0]
// 		]; 

// 		let parities: Vec<Vec<u64>> = vec![
// 		vec![0, 3, 4, 7], vec![0, 1, 6, 5], vec![1, 2, 5, 6], vec![2, 3, 4, 7]
// 		];  

// 		let mut decoder = Decoder::new(0, parities, symbols);
// 		println!("Decoder initialized.");

// 		// let mut symbol_arrive: Vec<u64> = (0..8).collect();
// 		// symbol_arrive.shuffle(&mut thread_rng());
// 		let symbol_arrive: Vec<u64> = vec![3, 5, 7, 0, 4, 2, 6, 1];
// 		let mut count = 0;
// 		println!("Checkpoint.");

// 		loop {
// 			let (symbols, symbol_indices, decoded) = decoder.symbol_update_from_reception(
// 				vec![Symbol::Base([0u8;BASE_SYMBOL_SIZE])], vec![symbol_arrive[count]]);
// 			if decoded {break;}
// 			if decoder.parity_update(symbols, symbol_indices) {
// 				if decoder.peeling_decode() {break;}
// 			}
// 			count += 1;
// 		}
// 		println!("Finish decoding using {} coded symbols.", count + 1);

// 		let mut flag = true;
// 		for i in 0..8 {			
// 			if let Symbol::Base(decoded) = decoder.symbol_values[i] {
// 				for j in 0..BASE_SYMBOL_SIZE {
// 					if decoded[j] != 0u8 {
// 						flag = false;
// 						break;
// 					}
// 				}				
// 			}
// 			if flag == false {break;}
// 		}
// 		assert_eq!(flag, true);
// 	}

// 	// #[test]
// 	// fn test_peeling_decoder2() {

// 	// }
// }




