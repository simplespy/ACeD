use super::crypto::{self};
use serde::{Serialize, Deserialize};
use super::block::{Transaction};
use super::hash::{H256};

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MerkleHash(pub H256);

//pub struct Symbol {
//    Base(H256),
//    Inter(H256),
//}
//
impl MerkleHash {
    pub fn new(transactions: &Vec<Transaction>) -> H256 {
        let mut root = H256::default();
        let default_hash = H256::default();
        let mut num_pad = 0;
        let len = transactions.len();

        if ! is_power_of_two(len) {
            let num_entry = (len as f64).log2().ceil();
            num_pad = (num_entry.exp2() as usize) - len;
        }

        merkle_hash(transactions, num_pad)
    }
}
    

pub fn is_power_of_two(x: usize) -> bool {
    (x != 0) && ((x & (x - 1)) == 0) 
}


pub fn merkle_hash(
    tx: &Vec<Transaction>, 
    num_pad: usize
) -> H256 {
    //let num_try = tx.len() + num_pad;
    //let num_layer = num_try.log2();
    //
    let mut base_hashes: Vec<H256>= vec![];
    for i in 0..tx.len() {
        let bytes: Vec<u8> = bincode::serialize(&tx[i]).unwrap();
        let hash = crypto::hash(&bytes);
        base_hashes.push(hash);
    }

    let pad_byte: Vec<u8> =bincode::serialize(&Transaction::default()).unwrap();  
    let pad_hash: H256 = crypto::hash(&pad_byte);
    
    for _ in 0..num_pad {
        base_hashes.push(pad_hash);
    }

    let mut layer_hashes = base_hashes;
    while layer_hashes.len() != 1 {
        let mut next_layer: Vec<H256> = vec![];
        for i in 0.. layer_hashes.len() {
            if i%2 == 0 {
                let mut bytes: Vec<u8> = layer_hashes[i].clone().into();
                bytes.extend_from_slice(&layer_hashes[i+1].clone().0);
                let hashof2 = crypto::hash(&bytes);
                next_layer.push(hashof2);
            } 
        }
        layer_hashes = next_layer;
    }
    layer_hashes[0]
}


