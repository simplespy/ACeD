use super::hash::{H256};
use super::block::{Block};
use std::sync::{Mutex, Arc};
use std::collections::{HashMap, VecDeque};
use super::cmtda::H256 as CMTH256;
use chain::block::Block as SBlock;
use chain::constants::{NUM_BASE_SYMBOL};
use super::network::message::{Samples};
use rocksdb::{self, ColumnFamilyDescriptor, Options, SliceTransform, DB};
use bincode::{deserialize, serialize};

const SYMBOL_CF: &str = "SYMBOL";
const BLOCK_CF: &str = "BLOCK";

pub struct BlockDb {
    pub block_record: VecDeque<u64>, // hack for reducing storage
    pub thresh: usize,
    pub num_sample: u64, // used by scale node
    pub num_block: u64,
    pub db: rocksdb::DB,
}

impl BlockDb {
    pub fn new<P: AsRef<std::path::Path>>(
        path: P,
    ) -> BlockDb {
        DB::destroy(&Options::default(), &path).unwrap();
        //let block_cf = ColumnFamilyDescriptor::new(BLOCK_CF, Options::default());
        let symbol_cf = ColumnFamilyDescriptor::new(SYMBOL_CF, Options::default());
        let cfs = vec![symbol_cf];
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open_cf_descriptors(&opts, path, cfs).unwrap();

        BlockDb {
            block_record: VecDeque::new(),
            thresh: 8,
            num_sample: 0,
            num_block: 0,
            db: db,
        }  
    }
    
    pub fn insert_sblock(&mut self, block_id: u64, sblock: SBlock){
        self.num_block += 1;
        //let block_cf = self.db.cf_handle(BLOCK_CF).unwrap();
        //let serialized = serialize(&sblock).unwrap();
        //let block_id = serialize(&block_id).unwrap();
        //self.db.put_cf(block_cf, &block_id, &serialized).unwrap(); 
    }

    pub fn get_sblock(&mut self, block_id: u64) -> Option<SBlock>{
        //let block_cf = self.db.cf_handle(BLOCK_CF).unwrap();
        //let block_id = serialize(&block_id).unwrap();
        //let serialized = self.db.get_pinned_cf(block_cf, &block_id).unwrap();
        //match serialized {
            //Some(block) => Some(deserialize(&block).unwrap()),
            //None => None,
        //}
        None
    }

    // return if there is redundant elements
    pub fn insert_cmt_sample(&mut self, block_id: u64 , chunk: &Samples) -> bool {
        let symbol_cf = self.db.cf_handle(SYMBOL_CF).unwrap();
        let serialized = serialize(&chunk).unwrap();
        let key = serialize(&block_id).unwrap();
        self.db.put_cf(symbol_cf, &key, &serialized).unwrap(); 
        self.num_sample += 1;
        self.block_record.push_back(block_id);
        // remove one block for saving storage
        if self.block_record.len() > self.thresh {
            let id = self.block_record.pop_front().unwrap();
            let id = serialize(&id).unwrap();
            self.db.delete(&id);
        }
        info!("curr staroge size {}", self.block_record.len());
        true
    }

    pub fn get_chunk(&self, block_id: u64) -> Option<Samples> {
        let symbol_cf = self.db.cf_handle(SYMBOL_CF).unwrap();
        let block_id = serialize(&block_id).unwrap();
        let serialized = self.db.get_pinned_cf(symbol_cf, &block_id).unwrap();
        match serialized {
            Some(chunk) => Some(deserialize(&chunk).unwrap()),
            None => None,
        }
    }

    pub fn get_num_blocks(&self) -> u64 {
       self.num_block as u64
    }

    pub fn get_num_samples(&self) -> u64 {
        self.num_sample as u64
    }
    
}
