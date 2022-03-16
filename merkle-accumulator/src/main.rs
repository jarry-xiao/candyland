use merkle::MerkleTree;
use rand::prelude::SliceRandom;
use rand::{thread_rng, random};
use rand::{self, Rng};
use solana_program::keccak::hashv;

mod merkle;
use crate::merkle::{empty_node, recompute, Node, MASK, MAX_DEPTH, MAX_SIZE, PADDING};

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
            roots: [empty_node(MAX_DEPTH as u32); MAX_SIZE],
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
            let j = self.active_index.wrapping_sub(i) & MASK;
            if self.roots[j as usize] != current_root {
                continue;
            }
            let old_root = recompute([0; 32], &proof, path);
            if old_root == current_root {
                return Some(self.update_and_apply_proof(leaf, &mut proof, path, j));
            } else {
                println!("Root mismatch {:?} {:?}", old_root, current_root);
                return None;
            }
        }
        if self.size == 0 {
            let old_root = recompute([0; 32], &proof, path);
            if old_root == empty_node(MAX_DEPTH as u32) {
                return Some(self.update_and_apply_proof(leaf, &mut proof, path, 0));
            } else {
                println!("Bad proof");
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
            let j = self.active_index.wrapping_sub(i) & MASK;

            if self.roots[j as usize] != current_root {
                if self.changes[j as usize].changes[MAX_DEPTH - 1] == leaf {
                    return None;
                }
                continue;
            }
            let old_root = recompute(leaf, &proof, path);
            if old_root == current_root {
                return Some(self.update_and_apply_proof([0; 32], &mut proof, path, j));
            } else {
                assert!(false);
                return None;
            }
        }
        println!("Failed to find root");
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
            let critbit_index = MAX_DEPTH
                - (((path ^ self.changes[j as usize].path) << PADDING).leading_zeros() as usize)
                - 1;
            proof[critbit_index] = self.changes[j as usize].changes[critbit_index];
        }
        if self.size > 0 {
            self.active_index += 1;
            self.active_index &= MASK;
        }
        if self.size < MAX_SIZE as u64 {
            self.size += 1;
        }
        let new_root = self.apply_changes(leaf, proof, path, self.active_index as usize);
        self.roots[self.active_index as usize] = new_root;
        new_root
    }

    fn apply_changes(&mut self, mut start: Node, proof: &[Node], path: u32, i: usize) -> Node {
        let change_log = &mut self.changes[i];
        change_log.changes[0] = start;
        for (ix, s) in proof.iter().enumerate() {
            if path >> ix & 1 == 1 {
                let res = hashv(&[&start, s.as_ref()]);
                start.copy_from_slice(res.as_ref());
            } else {
                let res = hashv(&[s.as_ref(), &start]);
                start.copy_from_slice(res.as_ref());
            }
            if ix < MAX_DEPTH - 1 {
                change_log.changes[ix + 1] = start;
            }
        }
        change_log.path = path;
        start
    }
}

fn main() {
    let mut rng = thread_rng();
    let mut leaves = vec![];
    let mut merkle = MerkleAccumulator::new();
    for _ in 0..(1 << MAX_DEPTH) {
        let leaf = [0; 32];
        leaves.push(leaf);
    }
    let mut uc_merkley = MerkleTree::new(leaves);
    println!("start root {:?}", uc_merkley.root);
    println!("start root {:?}", merkle.get());
    for i in 0..(1 << MAX_DEPTH - 1) {
        let leaf = rng.gen::<Node>();
        let (proof_vec, path) = uc_merkley.get_proof(i);
        let mut proof = [[0; 32]; MAX_DEPTH];
        for (i, x) in proof_vec.iter().enumerate() {
            proof[i] = *x;
        }
        merkle.add(uc_merkley.root, leaf, proof, path);
        uc_merkley.add_leaf(leaf, i);
    }

    println!("end root {:?}", uc_merkley.root);
    println!("end root {:?}", merkle.get());

    let mut proofs = vec![];
    let mut indices = vec![];

    let root = merkle.get();
    let mut inds: Vec<usize> = (0..(1 << MAX_DEPTH)).collect();
    inds.shuffle(&mut rng);

    for i in inds.into_iter().take(63) {
        if indices.iter().any(|j| *j == i) {
            continue;
        }
        let (proof_vec, path) = uc_merkley.get_proof(i);
        let mut proof = [[0; 32]; MAX_DEPTH];
        for (i, x) in proof_vec.iter().enumerate() {
            proof[i] = *x;
        }
        proofs.push((i, uc_merkley.get_node(i), proof, path));
        indices.push(i);
    }

    for (i, leaf, proof, path) in proofs.iter() {
        if *leaf != [0; 32] {
            println!("Remove {}", i);
            merkle.remove(root, uc_merkley.get_node(*i), *proof, *path);
            uc_merkley.remove_leaf(*i);
        } else {
            println!("Add {}", i);
            let random_leaf = rng.gen::<Node>();
            merkle.add(root, random_leaf, *proof, *path);
            uc_merkley.add_leaf(random_leaf, *i);
        }
        assert_eq!(merkle.get(), uc_merkley.root);
    }

    println!("end root {:?}", uc_merkley.root);
    println!("end root {:?}", merkle.get());
}
