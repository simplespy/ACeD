extern crate crypto;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use super::primitive::hash::{H256};

pub fn hash(input: &[u8]) -> H256 {
    let mut hash = H256::default();
    let mut hasher = Sha256::new();
    hasher.input(input);
    hasher.result(&mut hash.0[..]);
    hash
}

