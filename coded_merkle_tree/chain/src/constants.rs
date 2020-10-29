
// Below flags apply in the context of BIP 68
// If this flag set, CTxIn::nSequence is NOT interpreted as a
// relative lock-time.
pub const SEQUENCE_LOCKTIME_DISABLE_FLAG: u32 = 1u32 << 31;

// Setting nSequence to this value for every input in a transaction
// disables nLockTime.
pub const SEQUENCE_FINAL: u32 = 0xffffffff;

// If CTxIn::nSequence encodes a relative lock-time and this flag
// is set, the relative lock-time has units of 512 seconds,
// otherwise it specifies blocks with a granularity of 1.
pub const SEQUENCE_LOCKTIME_TYPE_FLAG: u32 = (1 << 22);

// If CTxIn::nSequence encodes a relative lock-time, this mask is
// applied to extract that lock-time from the sequence field.
pub const SEQUENCE_LOCKTIME_MASK: u32 = 0x0000ffff;

/// Threshold for `nLockTime`: below this value it is interpreted as block number,
/// otherwise as UNIX timestamp.
pub const LOCKTIME_THRESHOLD: u32 = 500000000; // Tue Nov  5 00:53:20 1985 UTC

/// Number of Satoshis in single coin
pub const SATOSHIS_IN_COIN: u64 = 100_000_000;

pub const UNDECODABLE_RATIO: f32 = 0.9; // > 1 - 0.124


//Configuration file for construction of coded Merkle tree

//size of transactions in a block in bytes
pub const BLOCK_SIZE: u64 = 16777216;//4194304;//16777216;//8388608;//4194304 //65536;//;16384  131072//131072;//131072;//;//131072; //32768;//10000;//131072; 40000 65535 4800 65535

pub const TRANSACTION_SIZE: u64 = 316;

//size of a symbol on the base layer in bytes
pub const BASE_SYMBOL_SIZE: usize = 131072;//32768;//131072;//65536;//32768;//32768;//32768;//1024;//32768;//1024;

//number of hashes to aggregate to form a new symbol on the upper layers of CMT
pub const AGGREGATE: usize = 8;

//coding rate for code ensemble
pub const RATE: f32 = 0.25;

pub const NUM_BASE_SYMBOL: u64 = (BLOCK_SIZE/(BASE_SYMBOL_SIZE as u64)) * ((1.0/RATE) as u64);

//number of hashes of coded symbols stored in the block header 
pub const HEADER_SIZE: u32 = 16;

//number of times to sample the base symbols of the CMT
pub const NUMBER_ITERATION: u32 = 10;

//number of coded symbols sampled by a light node to check data availability
pub const SAMPLE_COMPLEXITY: u32 = 30;








