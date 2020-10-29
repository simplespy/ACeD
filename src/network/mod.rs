extern crate mio_extras;
extern crate log;
pub mod server;
pub mod peer;
pub mod message;
pub mod performer;

use super::primitive;
use super::db::{blockDb, utxoDb};
use super::blockchain;
use super::contract;
use super::mempool;
use super::crypto;
use super::mempool::scheduler;
use super::cmtda;


// should be greater than coded_merkle_tree/chain constants.rs BLOCK_SIZE
pub const MSG_BUF_SIZE: usize = 1000_000;//65535;
