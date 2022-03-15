use solana_program::keccak::hashv;
use rand::thread_rng;
use rand::Rng;
// use rand::distributions::{Distribution, Standard};

const MAX_SIZE: usize = 64;
const MAX_DEPTH: usize = 20;
const PADDING: usize = 12;
const MASK: u64 = MAX_SIZE as u64 - 1;

type Node = [u8; 32];

fn recompute(mut start: Node, path: &[Node], address: u32) -> Node {
    for (ix, s) in path.iter().enumerate() {
        if address >> ix & 1 == 1 {
            let res = hashv(&[&start, s.as_ref()]);
            start.copy_from_slice(res.as_ref());
        } else {
            let res = hashv(&[s.as_ref(), &start]);
            start.copy_from_slice(res.as_ref());
        }
    }
    start
}

#[derive(Copy, Clone)]
pub struct ChangeLog {
    changes: [Node; MAX_DEPTH],
    path: u32,
}

pub struct MerkleAccumulator {
    roots: [Node; MAX_SIZE],
    changes: [ChangeLog; MAX_SIZE],
    active_index: u64,
    size: u64,
}

impl MerkleAccumulator {
    pub fn new() -> Self {
        Self {
            roots: [[0; 32]; MAX_SIZE],
            changes: [ChangeLog {
                changes: [[0; 32]; MAX_DEPTH],
                path: 0,
            }; MAX_SIZE],
            active_index: 0,
            size: 0,
        }
    }

    pub fn get(&self) -> Node {
        self.roots[self.active_index as usize]
    }

    pub fn add(
        &mut self,
        current_root: Node,
        leaf: Node,
        mut proof: [Node; MAX_DEPTH],
        path: u32,
    ) -> Option<Node> {
        for i in 0..self.size {
            let j = (self.active_index - i) & MASK;
            if self.roots[j as usize] != current_root {
                continue;
            }
            let old_root = recompute([0; 32], &proof, path);
            if old_root == current_root {
                return Some(self.update_and_apply_proof(leaf, &mut proof, path, j));
            } else {
                return None;
            }
        }
        return None;
    }

    pub fn remove(
        &mut self,
        current_root: Node,
        leaf: Node,
        mut proof: [Node; MAX_DEPTH],
        path: u32,
    ) -> Option<Node> {
        for i in 0..self.size {
            let j = (self.active_index - i) & MASK;

            if self.roots[j as usize] != current_root {
                if self.changes[j as usize].changes[MAX_DEPTH - 1] == leaf {
                    return None;
                }
                continue;
            }
            let old_root = recompute(leaf, &proof, path);
            if old_root == current_root {
                return Some(self.update_and_apply_proof(leaf, &mut proof, path, j));
            } else {
                return None;
            }
        }
        return None;
    }

    fn update_and_apply_proof(
        &mut self,
        leaf: Node,
        proof: &mut [Node; MAX_DEPTH],
        path: u32,
        mut j: u64,
    ) -> Node {
        while j != self.active_index {
            j += 1;
            j &= MASK;
            let critbit_index =
                (path ^ self.changes[j as usize].path).leading_zeros() as usize - PADDING;
            proof[critbit_index] = self.changes[j as usize].changes[critbit_index];
        }
        self.active_index += 1;
        self.active_index &= MASK;
        if self.size < MAX_SIZE as u64 {
            self.size += 1;
        }
        let new_root = self.apply_changes(leaf, proof, path, self.active_index as usize);
        self.roots[self.active_index as usize] = new_root;
        new_root
    }

    fn apply_changes(&mut self, mut start: Node, proof: &[Node], path: u32, i: usize) -> Node {
        let change_log = &mut self.changes[i];
        change_log.changes[MAX_DEPTH - 1] = start;
        for (ix, s) in proof.iter().enumerate() {
            if path >> ix & 1 == 1 {
                let res = hashv(&[&start, s.as_ref()]);
                start.copy_from_slice(res.as_ref());
            } else {
                let res = hashv(&[s.as_ref(), &start]);
                start.copy_from_slice(res.as_ref());
            }
            if ix <= MAX_DEPTH - 2 {
                change_log.changes[MAX_DEPTH - 2 - ix] = start;
            }
        }
        change_log.path = path;
        start
    }
}

fn main() {
    let mut rng = thread_rng();

    let mut v = vec![];
    let mut merkle = MerkleAccumulator::new();
    for _ in 0..128 {
        let leaf = rng.gen::<[u8; 32]>();
//        merkle.add(leaf);
        v.push(leaf);
        println!("leaf {:?}, root {:?}", leaf, merkle.get());
    }
}
