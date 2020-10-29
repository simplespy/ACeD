use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicUsize, AtomicU64, AtomicU32, Ordering, AtomicBool};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

lazy_static! {
    pub static ref PERFORMANCE_COUNTER: Counter = { Counter::default() };
}

#[derive(Default)]
pub struct Counter {
    scale_id: AtomicUsize,
    generated_transactions: AtomicUsize,
    confirmed_transactions: AtomicUsize,
    chain_depth: AtomicUsize,
    token: AtomicBool,

    propose_block: AtomicUsize, // side node block id
    sign_block: AtomicUsize, // scale node # block id 0 for idle
    submit_block: AtomicUsize,   // is submitting blocks id 0 for idle
    block: AtomicU64,
    coll_block: AtomicUsize,

    propose_sec: AtomicU64,
    propose_millis: AtomicU32,
    propose_num: AtomicUsize,
    propose_anc: AtomicUsize,

    sign_loaded: AtomicBool,
    sign_sec: AtomicU64,
    sign_millis: AtomicU32,
    sign_num: AtomicU64,
    sign_anc: AtomicU64,

    submit_loaded: AtomicBool,
    submit_sec: AtomicU64,
    submit_millis: AtomicU32,
    submit_num: AtomicU64,
    submit_anc: AtomicU64,

    block_loaded: AtomicBool,
    block_sec: AtomicU64,
    block_millis: AtomicU32,
    block_num: AtomicU64,

    coll_loaded: AtomicBool,
    coll_sec: AtomicU64,
    coll_millis: AtomicU32,
    coll_num: AtomicU64,

    propose_latency: AtomicUsize,
    sign_latency: AtomicUsize,
    submit_latency: AtomicUsize,   // time taken to submit
    coll_latency: AtomicUsize,

    block_latency: AtomicUsize,

    gas: AtomicUsize,

}

impl Counter {
    pub fn record_scale_id(&self, scale_id: usize) {
        self.scale_id.store(scale_id, Ordering::Relaxed);
    }

    pub fn record_generated_transaction(&self) {
        self.generated_transactions.fetch_add(1, Ordering::Relaxed); 
    }
    pub fn record_confirmeded_transaction(&self) {
        self.confirmed_transactions.fetch_add(1, Ordering::Relaxed); 
    }

    pub fn record_generated_transactions(&self, num: usize) {
        self.generated_transactions.fetch_add(num, Ordering::Relaxed); 
    }

    pub fn record_confirmeded_transactions(&self, num: usize) {
        self.confirmed_transactions.fetch_add(num, Ordering::Relaxed); 
    }

    pub fn record_chain_update(&self) {
        self.chain_depth.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_gas_update(&self, gas: usize ) {
        self.gas.fetch_add(gas, Ordering::Relaxed);
    }

    // should not be used later
    pub fn store_chain_depth(&self, chain_len: usize) {
        self.chain_depth.store(chain_len, Ordering::Relaxed);
    }

    pub fn record_token_update(&self, new_flag: bool) {
        self.token.store(new_flag, Ordering::Relaxed);
    }

    fn get_times(&self) -> (u64, u32) {
        let dur = SystemTime::now().
            duration_since(SystemTime::UNIX_EPOCH).
            unwrap();
        let sec = dur.as_secs();   
        let millis = dur.subsec_millis();
        (sec, millis)
    }

    fn subtract_times(&self, sec: u64, millis: u32, psec: u64, pmillis: u32) -> usize {
        ((sec - psec) * 1000) as usize + millis as usize - pmillis as usize
    }

    pub fn record_propose_block_update(&self, id: u64) {
    
        let (sec, millis) = self.get_times();
        self.propose_sec.store(sec, Ordering::Relaxed);
        self.propose_millis.store(millis, Ordering::Relaxed); 
    }

    pub fn record_propose_block_id(&self, id: usize) {
        self.propose_block.store(id, Ordering::Relaxed);
    }
    
    pub fn record_propose_block_stop(&self) {
        let (sec, millis) = self.get_times();
        let psec = self.propose_sec.load(Ordering::Relaxed);
        let pmillis = self.propose_millis.load(Ordering::Relaxed);
        let lat = self.subtract_times(sec, millis, psec, pmillis);
        self.propose_latency.fetch_add(lat, Ordering::Relaxed);
        self.propose_num.fetch_add(1, Ordering::Relaxed);
        info!("propse lat lat {}", lat);
    }

    pub fn record_sign_block_update(&self, id: u64) {
        self.sign_block.store(id as usize, Ordering::Relaxed);
        if !self.sign_loaded.compare_and_swap(false, true, Ordering::Relaxed) {
            self.sign_anc.store(id , Ordering::Relaxed);
            let (sec, millis) = self.get_times();
            self.sign_sec.store(sec, Ordering::Relaxed);
            self.sign_millis.store(millis, Ordering::Relaxed); 
        }
    }

    pub fn record_sign_block_stop(&self, id: usize) {
        if id as u64 == self.sign_anc.load(Ordering::Relaxed) {
            let (sec, millis) = self.get_times();
            let psec = self.sign_sec.load(Ordering::Relaxed);
            let pmillis = self.sign_millis.load(Ordering::Relaxed);
            let lat = self.subtract_times(sec, millis, psec, pmillis);
            self.sign_latency.fetch_add(lat, Ordering::Relaxed);
             self.sign_num.fetch_add(1, Ordering::Relaxed);           
            self.sign_loaded.store(false, Ordering::Relaxed);
            info!("sign latency block id {} lat {}", id, lat);
        }
    }

    pub fn record_submit_block_update(&self, id: u64) {
        self.submit_block.store(id as usize, Ordering::Relaxed);
        if !self.submit_loaded.compare_and_swap(false, true, Ordering::Relaxed) {
            self.submit_anc.store(id, Ordering::Relaxed);   
            let (sec, millis) = self.get_times();
            self.submit_sec.store(sec, Ordering::Relaxed);
            self.submit_millis.store(millis, Ordering::Relaxed); 
        }
    }

    pub fn record_submit_block_stop(&self, id: usize) {
        if id as u64 == self.submit_anc.load(Ordering::Relaxed) {
            let (sec, millis) = self.get_times();
            let psec = self.submit_sec.load(Ordering::Relaxed);
            let pmillis = self.submit_millis.load(Ordering::Relaxed);
            let lat = self.subtract_times(sec, millis, psec, pmillis);
            self.submit_latency.fetch_add(lat, Ordering::Relaxed);
            self.submit_num.fetch_add(1, Ordering::Relaxed);
            self.submit_loaded.store(false, Ordering::Relaxed);
            info!("submit latency block id {} lat {}", id, lat);
        }
    }

    pub fn record_block_update(&self, id: u64) {
        if !self.block_loaded.compare_and_swap(false, true, Ordering::Relaxed) {
            self.block.store(id, Ordering::Relaxed);
            let (sec, millis) = self.get_times();
            self.block_sec.store(sec, Ordering::Relaxed);
            self.block_millis.store(millis, Ordering::Relaxed); 
        }
    }

    pub fn record_block_stop(&self, id: u64) {
        if id == self.block.load(Ordering::Relaxed) {
            self.block.store(0, Ordering::Relaxed);
            let (sec, millis) = self.get_times();
            let psec = self.block_sec.load(Ordering::Relaxed);
            let pmillis = self.block_millis.load(Ordering::Relaxed);
            let lat = self.subtract_times(sec, millis, psec, pmillis);
            self.block_latency.fetch_add(lat, Ordering::Relaxed);
            self.block_num.fetch_add(1, Ordering::Relaxed);
            self.block_loaded.store(false, Ordering::Relaxed);
            info!("block id {} lat {}", id, lat);
        }
    }

    pub fn record_coll_block_update(&self, id: u64) {
        if !self.coll_loaded.compare_and_swap(false, true, Ordering::Relaxed) {
            self.coll_block.store(id as usize, Ordering::Relaxed);
            let (sec, millis) = self.get_times();
            self.coll_sec.store(sec, Ordering::Relaxed);
            self.coll_millis.store(millis, Ordering::Relaxed); 
        }
    }

    pub fn record_coll_block_stop(&self, id: usize) {
        if id == self.coll_block.load(Ordering::Relaxed) {
            let (sec, millis) = self.get_times();
            let psec = self.coll_sec.load(Ordering::Relaxed);
            let pmillis = self.coll_millis.load(Ordering::Relaxed);
            let lat = self.subtract_times(sec, millis, psec, pmillis);
            //info!("coll id {} lat {}", id, lat);
            self.coll_latency.fetch_add(lat, Ordering::Relaxed);
            self.coll_num.fetch_add(1, Ordering::Relaxed);
            self.coll_loaded.store(false, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            scale_id: self.scale_id.load(Ordering::Relaxed),
            generated_transactions: self.generated_transactions.load(Ordering::Relaxed),
            confirmed_transactions: self.confirmed_transactions.load(Ordering::Relaxed),
            chain_depth: self.chain_depth.load(Ordering::Relaxed),
            token: self.token.load(Ordering::Relaxed),
            propose_block: self.propose_block.load(Ordering::Relaxed),
            sign_block: self.sign_block.load(Ordering::Relaxed),
            submit_block: self.submit_block.load(Ordering::Relaxed),
            coll_block: self.coll_block.load(Ordering::Relaxed),
            propose_latency: self.propose_latency.load(Ordering::Relaxed),
            sign_latency: self.sign_latency.load(Ordering::Relaxed),
            submit_latency: self.submit_latency.load(Ordering::Relaxed),
            block_latency: self.block_latency.load(Ordering::Relaxed),
            coll_latency: self.coll_latency.load(Ordering::Relaxed),
            gas: self.gas.load(Ordering::Relaxed),
            propose_num: self.propose_num.load(Ordering::Relaxed),
            sign_num: self.sign_num.load(Ordering::Relaxed) as usize,
            submit_num: self.submit_num.load(Ordering::Relaxed) as usize,
            block_num: self.block_num.load(Ordering::Relaxed) as usize,
            coll_num: self.coll_num.load(Ordering::Relaxed) as usize,
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Snapshot {
    scale_id:                usize,
    generated_transactions:  usize,
    confirmed_transactions:  usize,
    chain_depth:             usize,
    token:                   bool,

    propose_block:           usize,
    sign_block:              usize, // scale node # block id 0 for idle
    submit_block:            usize, 
    coll_block:              usize,

    propose_latency:         usize,
    sign_latency:            usize,
    submit_latency:          usize,
    block_latency:           usize,
    coll_latency:            usize,

    gas:                     usize,

    propose_num:             usize,
    sign_num:                usize,
    submit_num:              usize,
    block_num:               usize,
    coll_num:                usize,
}
