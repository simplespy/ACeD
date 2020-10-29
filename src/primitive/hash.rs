extern crate rand;
use rand::Rng;

use std::fmt;
use serde::{Serialize, Deserialize};
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct H256(pub [u8; 32]);

impl fmt::Debug for H256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:X}{:X}..{:X}{:X}", 
            self.0[0],
            self.0[1],
            self.0[30],
            self.0[31],
        ) 
    }
}

impl H256 {
    pub fn random(&mut self) {
        for i in 0..32 {
            let r: u8 = rand::thread_rng().gen();
            self.0[i] = r;
        }
    }

    pub fn new() -> H256 {
        let mut h256 = H256::default();
        h256.random();
        h256
    }

    pub fn zero() -> H256 {
        let mut h256 = H256::default();
        for i in 0..32 {
            h256.0[i] = 0;
        }
        h256
    }
}

impl fmt::Display for H256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte_idx in 0..32 {
            write!(f, "{:>02x}", &self.0[byte_idx])?;
        }
        Ok(())
    }
}

impl Default for H256 {
    fn default() -> H256 {
        H256([0 as u8; 32]) 
    }
}

impl Hash for H256 {
    fn hash<H: Hasher>(&self, state: &mut H)  {
        state.write(&self.0);
    }
}

impl From<H256> for [u8; 32] {
    fn from(h: H256) -> [u8; 32] {
        h.0 
    }
}

impl AsRef<H256> for H256 {
    fn as_ref(&self) -> &H256 {
        self 
    }
}

impl From<H256> for Vec<u8> {
    fn from(h: H256) -> Vec<u8> {
        h.0.to_vec()
    }
}

impl From<web3::types::H256> for H256 {
    fn from(h: web3::types::H256) -> Self {
        H256 ( *h.as_fixed_bytes() )
    }
}


impl From<String> for H256 {
    fn from(h: String) -> H256 {
        let h = h.replace("0x", "");
        let h = hex::decode(&h).unwrap();
        let mut tx_hash: [u8; 32] = [0; 32];
        tx_hash.copy_from_slice(h.as_ref());
        H256 ( tx_hash)
    }
}
