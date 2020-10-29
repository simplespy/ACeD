use super::block::{Block, Header};
use super::hash::{H256};
use std::collections::{HashSet, HashMap};
use petgraph::{self, Direction};
use petgraph::graph::{NodeIndex};
use petgraph::stable_graph::{StableGraph};
use petgraph::visit::EdgeRef;

pub struct ForkBuffer {
    pub graph: StableGraph::<Header, ()>, 
    pub hash_node: HashMap<H256, NodeIndex>,
    //map of prevhash to its containing node, incase no parent
    pub prev_hash_to_nodes: HashMap<H256, Vec<NodeIndex>>, 
    pub hash_contact_height: HashMap<H256, usize>,
    pub leaf_hashes: HashSet<H256>,
}

impl ForkBuffer {
    pub fn new() -> ForkBuffer {
        ForkBuffer {
            graph: StableGraph::<Header, ()>::new(),
            hash_node: HashMap::new(),
            prev_hash_to_nodes: HashMap::new(),
            hash_contact_height: HashMap::new(),
            leaf_hashes: HashSet::new(),
        } 
    }

    pub fn record_contact_height(&mut self, hash: H256, height: usize) {
        self.hash_contact_height.insert(hash, height); 
    }

    // return longest chain
    pub fn insert(
        &mut self, 
        header: &Header,
        block_height: usize,
    ) {
        let block_hash = header.hash;
        let prev_hash = &header.prev_hash;
        match self.hash_node.get(&block_hash) {
            None => {
                // register nodes
                let node = self.graph.add_node(header.clone());
                self.hash_node.insert(block_hash, node);
                
                // connect to its parent
                match self.hash_node.get(&prev_hash) {
                    Some(prev_node) => {
                        self.graph.add_edge(*prev_node, node, ()); 
                        self.leaf_hashes.remove(prev_hash);
                    },
                    None =>  {
                        let mut nodes = self.prev_hash_to_nodes.entry(*prev_hash).or_insert(vec![node]);
                        (*nodes).push(node);

                    },
                }
                //  connect to its children
                match self.prev_hash_to_nodes.get(&block_hash) {
                    None =>  {
                        self.leaf_hashes.insert(block_hash);
                    },
                    Some(next_nodes) => {
                        for n in next_nodes.iter() {
                            self.graph.add_edge(node, *n, ()); 
                        }
                        self.prev_hash_to_nodes.remove(&block_hash);
                    }
                }
            },
            _ => (),
        }
    }

    pub fn get_parent(&self, hash: &H256) -> Option<H256> {
        match self.hash_node.get(hash) {
            None => return None,
            Some(node) => {
                let mut in_edge = self.graph.edges_directed(
                    *node, 
                    Direction::Incoming
                );

                let mut num_in = 0;
                let mut parent_hash = H256::default();

                for e in in_edge {
                    let src_n = e.source();
                    parent_hash = self.graph[src_n].hash;
                }

                if num_in == 0 {
                    None
                } else if num_in == 1 {
                    Some(parent_hash) 
                } else{
                    panic!("more than 1 in edge");    
                }
            }
        }
    }

    pub fn get_parent_hashes(&self, leaf_hash: H256) -> Vec<H256> {
        let mut lower_hash = leaf_hash;
        let mut chain: Vec<H256> = vec![];
        chain.push(lower_hash);
        while let Some(hash) = self.get_parent(&lower_hash) {
            chain.push(hash); 
            lower_hash = hash;
        }
        chain 
    }

    // return longest chain passing the hash
    pub fn get_longest_chain_by_hash(
        &mut self, 
        main_chain_height: usize,
    ) -> Option<(usize, Vec<H256>)> {
        let mut longest_height = 0;
        let mut chain: Vec<H256> = vec![];
        let mut reset = false;
        let mut longest_contact_height = 0;

        for leaf_hash in &self.leaf_hashes {
            let leaf_chain = self.get_parent_hashes(*leaf_hash);     
            let head_hash = &leaf_chain[0];
            match self.hash_contact_height.get(head_hash) {
                None => (),
                Some(contact_height) => {
                    longest_height = *contact_height + leaf_chain.len() - 1;
                    if longest_height > main_chain_height {
                        reset = true;
                        chain = leaf_chain;
                        longest_contact_height = *contact_height;
                    } 
                },
            }
        } 
        if reset {
            return Some((longest_contact_height, chain));
        } else {
            return None;
        }
    }


    pub fn remove(&mut self, hash: &H256) {
        match self.hash_node.get(hash) {
            None => (),
            Some(node) => {
                 
            }
        } 
    }
}
