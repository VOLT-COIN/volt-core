use std::collections::LinkedList;
// use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

const K_BUCKET_SIZE: usize = 20;
const ID_SIZE: usize = 32; // SHA256 space

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    pub fn _new(data: [u8; 32]) -> Self {
        NodeId(data)
    }

    pub fn random() -> Self {
        use rand::RngCore;
        let mut data = [0u8; 32];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut data);
        NodeId(data)
    }

    pub fn distance(&self, other: &NodeId) -> NodeId {
        let mut dist = [0u8; 32];
        for (i, byte) in dist.iter_mut().enumerate() {
            *byte = self.0[i] ^ other.0[i];
        }
        NodeId(dist)
    }
    
    // Calculate leading zeros for distance comparisons
    pub fn leading_zeros(&self) -> u32 {
        let mut zeros = 0;
        for byte in &self.0 {
            if *byte == 0 {
                zeros += 8;
            } else {
                zeros += byte.leading_zeros();
                break;
            }
        }
        zeros
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Peer {
    pub id: NodeId,
    pub address: String, // IP:Port
    pub last_seen: u64,
}

pub struct Bucket {
    pub peers: LinkedList<Peer>,
}

impl Bucket {
    pub fn new() -> Self {
        Bucket { peers: LinkedList::new() }
    }

    pub fn add(&mut self, peer: Peer) -> Option<Peer> {
        // Check if exists
        if let Some(_pos) = self.peers.iter().position(|p| p.id == peer.id) {
            // Move to tail (most recently seen)
            // LinkedList remove is O(n), Vec might be better but K is small.
            // Simplified: Update last_seen if found, don't move for now to keep simple API.
            // Actually, Kademlia requires moving to tail.
            return None;
        }

        if self.peers.len() < K_BUCKET_SIZE {
            self.peers.push_back(peer);
            None
        } else {
            // Bucket full. Return head (least recently seen) to ping check.
            self.peers.front().cloned()
        }
    }
}

pub struct RoutingTable {
    pub local_id: NodeId,
    pub buckets: Vec<Bucket>, // 256 buckets for 256 bits of ID space
}

impl RoutingTable {
    pub fn new(local_id: NodeId) -> Self {
        let mut buckets = Vec::with_capacity(ID_SIZE * 8);
        for _ in 0..(ID_SIZE * 8) {
            buckets.push(Bucket::new());
        }
        
        RoutingTable {
            local_id,
            buckets,
        }
    }

    pub fn add_peer(&mut self, peer: Peer) {
        let bucket_idx = self.get_bucket_index(&peer.id);
        self.buckets[bucket_idx].add(peer);
    }
    
    fn get_bucket_index(&self, other_id: &NodeId) -> usize {
        let dist = self.local_id.distance(other_id);
        let zeros = dist.leading_zeros() as usize;
        if zeros >= (ID_SIZE * 8) {
            return (ID_SIZE * 8) - 1; // Self
        }
        zeros
    }

    pub fn find_closest(&self, target_id: &NodeId, count: usize) -> Vec<Peer> {
        // Collect peers from closest buckets
        let mut result = Vec::new();
        let target_idx = self.get_bucket_index(target_id);
        
        // Check target bucket
        for peer in &self.buckets[target_idx].peers {
             result.push(peer.clone());
        }

        // Spread out
        let mut offset = 1;
        while result.len() < count {
             let mut found_more = false;
             if target_idx >= offset {
                 for peer in &self.buckets[target_idx - offset].peers { 
                     result.push(peer.clone()); 
                     found_more = true;
                 }
             }
             if target_idx + offset < self.buckets.len() {
                 for peer in &self.buckets[target_idx + offset].peers { 
                     result.push(peer.clone()); 
                     found_more = true;
                 }
             }
             if !found_more && offset > self.buckets.len() { break; }
             offset += 1;
        }
        
        result.truncate(count);
        result
    }
}
