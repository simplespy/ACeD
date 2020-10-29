use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str;
pub use chain::block_header::{BlockHeader};
pub use chain::hash::H256;
pub use primitives::bytes::{Bytes};


pub use chain::transaction::{Transaction, TransactionInput, TransactionOutput, OutPoint};
pub use chain::block::Block;
pub use chain::constants::{BLOCK_SIZE, BASE_SYMBOL_SIZE, AGGREGATE, RATE, HEADER_SIZE, NUMBER_ITERATION};
pub use chain::coded_merkle_roots::{Symbols, SymbolBase, SymbolUp, coded_merkle_roots};
pub use chain::merkle_root::merkle_root;
pub use chain::decoder::{Code, Symbol, Decoder, TreeDecoder, CodingErr, IncorrectCodingProof};


// obtain a code represented by symbols from the form represented by parities
pub fn convert_parity_to_symbol(parities: Vec<Vec<u64>>, n: u64) -> Vec<Vec<u64>> {
	let mut symbols: Vec<Vec<u64>> = vec![vec![];n as usize];
	for i in 0..parities.len(){
		let parity = &parities[i];
		for s in parity.iter() {
			symbols[*s as usize].push(i as u64);
		}
	}
	symbols
}

//Read all codes for all coded Merkle tree layers
 pub fn read_codes(k_set: Vec<u64>, filepath: &str) -> (Vec<Code>, Vec<Code>) {
	let mut codes_for_encoding: Vec<Code> = vec![];
	let mut codes_for_decoding: Vec<Code> = vec![];
	for i in k_set.iter() {
		let (code_e, code_d) = read_code_from_file(*i, filepath);
		codes_for_encoding.push(code_e);
		codes_for_decoding.push(code_d);
	}
	(codes_for_encoding, codes_for_decoding)
}

pub fn read_code_from_file(k: u64, filepath: &str) -> (Code, Code) {
    //compute number of coded symbols
	let n = ((k as f32) / RATE ) as u64;
	//Read encoding matrix
	let filename = String::from(filepath) + "/k=" +  &k.to_string() + &String::from("_encode.txt");
    // Open the file in read-only mode (ignoring errors).
    let file = File::open(filename).expect("unable to read file");
    let reader = BufReader::new(file);
    
    //parity equations for encoding
    let mut parities_encoding: Vec<Vec<u64>> = vec![];
    for (index, line) in reader.lines().enumerate() {
        let line = line.unwrap(); // Ignore errors.
        let parity: Vec<u64> = line.split_whitespace().map(|s| s.parse().unwrap()).collect();
        parities_encoding.push(parity);
    }

    //Read decodeing matrix
    let filename = String::from(filepath) + "/k=" + &k.to_string() + &String::from("_decode.txt");
    // Open the file in read-only mode (ignoring errors).
    let file = File::open(filename).expect("unable to deocding matrix");
    let reader = BufReader::new(file);
    
    //parity equations for decoding
    let mut parities_decoding: Vec<Vec<u64>> = vec![];
    for (index, line) in reader.lines().enumerate() {
        let line = line.unwrap(); // Ignore errors.
        let parity: Vec<u64> = line.split_whitespace().map(|s| s.parse().unwrap()).collect();
        parities_decoding.push(parity);
    }
    
    //Generate one code for encoding, and one code for decoding
    (Code {parities: parities_encoding.clone(), symbols: convert_parity_to_symbol(parities_encoding, n)}, 
    	Code {parities: parities_decoding.clone(), symbols: convert_parity_to_symbol(parities_decoding, n)})
}


