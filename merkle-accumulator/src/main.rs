use solana_program::keccak::hashv;
mod merkle;
use crate::merkle::{empty_node, recompute, Node, MASK, MAX_DEPTH, MAX_SIZE, PADDING};

#[derive(Copy, Clone)]
/// Stores proof for a given Merkle root update
pub struct ChangeLog {
    /// Nodes of off-chain merkle tree
    changes: [Node; MAX_DEPTH],
    /// Bitmap of node parity (used when hashing)
    path: u32,
}

/// Tracks updates to off-chain Merkle tree
///
/// Allows for concurrent writes to same merkle tree so long as proof
/// was generated for a that has had at most MAX_SIZE updates since the tx was submitted
pub struct MerkleAccumulator {
    /// Chronological roots of the off-chain Merkle tree stored in circular buffer
    roots: [Node; MAX_SIZE],
    /// Proof for respective root
    change_logs: [ChangeLog; MAX_SIZE],
    /// Index of most recent root & changes
    active_index: usize,
    /// Number of active changes we are tracking
    buffer_size: usize,
}

impl MerkleAccumulator {
    pub fn new() -> Self {
        Self {
            roots: [empty_node(MAX_DEPTH as u32); MAX_SIZE],
            change_logs: [ChangeLog {
                changes: [[0; 32]; MAX_DEPTH],
                path: 0,
            }; MAX_SIZE],
            active_index: 0,
            buffer_size: 0,
        }
    }

    pub fn new_with_root(root: Node) -> Self {
        let mut roots = [empty_node(MAX_DEPTH as u32); MAX_SIZE];
        roots[0] = root;
        Self {
            roots,
            change_logs: [ChangeLog {
                changes: [[0; 32]; MAX_DEPTH],
                path: 0,
            }; MAX_SIZE],
            active_index: 0,
            buffer_size: 1,
        }
    }

    /// Returns on-chain root
    pub fn get(&self) -> Node {
        self.roots[self.active_index]
    }

    pub fn add(
        &mut self,
        current_root: Node,
        leaf: Node,
        mut proof: [Node; MAX_DEPTH],
        path: u32,
    ) -> Option<Node> {
        if self.buffer_size == 0 {
            let old_root = recompute([0; 32], &proof, path);
            if old_root == empty_node(MAX_DEPTH as u32) {
                return Some(self.update_and_apply_proof(leaf, &mut proof, path, 0));
            } else {
                println!("Bad proof");
                return None;
            }
        }
        self.replace(current_root, [0; 32], leaf, proof, path)
    }

    pub fn remove(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof: [Node; MAX_DEPTH],
        path: u32,
    ) -> Option<Node> {
        self.replace(current_root, leaf, [0; 32], proof, path)
    }

    pub fn replace(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        mut proof: [Node; MAX_DEPTH],
        path: u32,
    ) -> Option<Node> {
        for i in 0..self.buffer_size {
            let j = self.active_index.wrapping_sub(i) & MASK;

            if self.roots[j] != current_root {
                if self.change_logs[j].changes[MAX_DEPTH - 1] == leaf {
                    return None;
                }
                continue;
            }
            let old_root = recompute(leaf, &proof, path);
            if old_root == current_root {
                return Some(self.update_and_apply_proof(new_leaf, &mut proof, path, j));
            } else {
                assert!(false);
                return None;
            }
        }
        println!("Failed to find root");
        None
    }

    /// Fast-forwards submitted proof to be valid for the root at `self.current_index`
    ///
    /// Updates proof & updates root & stores
    ///
    /// Takes in `j`, which is the root index that this proof was last valid for
    fn update_and_apply_proof(
        &mut self,
        leaf: Node,
        proof: &mut [Node; MAX_DEPTH],
        path: u32,
        mut j: usize,
    ) -> Node {
        while j != self.active_index {
            j += 1;
            j &= MASK;
            let path_len = ((path ^ self.change_logs[j].path) << PADDING).leading_zeros() as usize;
            let critbit_index = (MAX_DEPTH - 1) - path_len;
            proof[critbit_index] = self.change_logs[j].changes[critbit_index];
        }
        if self.buffer_size > 0 {
            self.active_index += 1;
            self.active_index &= MASK;
        }
        if self.buffer_size < MAX_SIZE {
            self.buffer_size += 1;
        }
        let new_root = self.apply_changes(leaf, proof, path, self.active_index);
        self.roots[self.active_index] = new_root;
        new_root
    }

    /// Creates a new root from a proof that is valid for the root at `self.active_index`
    /// Saves hashed nodes for new root in change log
    fn apply_changes(&mut self, mut start: Node, proof: &[Node], path: u32, i: usize) -> Node {
        let change_log = &mut self.change_logs[i];
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
    println!("Hello world!");
}

#[cfg(test)]
mod test {
    use super::{merkle::*, MerkleAccumulator};
    use rand::prelude::SliceRandom;
    use rand::{self, Rng};
    use rand::{rngs::ThreadRng, thread_rng};

    /// Initializes off-chain Merkle Tree & creates on-chain tree
    #[inline]
    fn setup() -> (MerkleAccumulator, MerkleTree) {
        // Setup
        let mut leaves = vec![];
        // on-chain merkle change-record
        let merkle = MerkleAccumulator::new();

        // Init off-chain Merkle tree with leaves
        for _ in 0..(1 << MAX_DEPTH) {
            let leaf = [0; 32];
            leaves.push(leaf);
        }
        // off-chain merkle tree
        let uc_merkley = MerkleTree::new(leaves);

        (merkle, uc_merkley)
    }

    /// Adds random leaves to on-chain & records off-chain
    /// sync_on_chain: False if we are using new_with_root()
    fn add_random_leafs(
        merkle: &mut MerkleAccumulator,
        off_chain_merkle: &mut MerkleTree,
        rng: &mut ThreadRng,
        num: usize,
    ) {
        for i in 0..num {
            let leaf = rng.gen::<Node>();
            let (proof_vec, path) = off_chain_merkle.get_proof_of_leaf(i);

            merkle.add(off_chain_merkle.root, leaf, proof_to_slice(proof_vec), path);
            off_chain_merkle.add_leaf(leaf, i);
        }
    }

    fn proof_to_slice(proof_vec: Vec<Node>) -> [Node; MAX_DEPTH] {
        let mut slice = [[0; 32]; MAX_DEPTH];
        for (i, x) in proof_vec.iter().enumerate() {
            slice[i] = *x;
        }
        slice
    }

    /// Creates proofs of leaves in off-chain merkle to be written to on-chain merkle accumulator
    #[inline]
    fn create_proofs_of_existence(
        _merkle: &MerkleAccumulator,
        off_chain_merkle: &MerkleTree,
        rng: &mut ThreadRng,
        num_leaves: usize,
    ) -> (Vec<(usize, Node, [Node; MAX_DEPTH], u32)>, Vec<usize>) {
        let mut inds: Vec<usize> = (0..num_leaves).collect();
        inds.shuffle(rng);
        let mut proofs = vec![];
        let mut indices = vec![];

        for i in inds.into_iter().take(MAX_SIZE - 1) {
            let (proof_vec, path) = off_chain_merkle.get_proof_of_leaf(i);

            // Make on-chain readable proof
            let proof = proof_to_slice(proof_vec);

            proofs.push((i, off_chain_merkle.get_node(i), proof, path));
            indices.push(i);
        }
        (proofs, indices)
    }

    // Test: add_leaf
    // ------
    // Note: we are not initializing on-chain merkle accumulator, we just start using it to track changes
    // Off-chain: replace 1st half of leaves with random values
    // On-chain: record updates to the root
    #[test]
    fn test_add_all() {
        let (mut merkle, mut off_chain_merkle) = setup();
        let mut rng = thread_rng();

        println!("Accumulator init root     : {:?}", merkle.get());
        println!("Off-chain merkle init root: {:?}", off_chain_merkle.root);

        add_random_leafs(&mut merkle, &mut off_chain_merkle, &mut rng, 1 << MAX_DEPTH);

        assert_eq!(
            merkle.get(),
            off_chain_merkle.root,
            "Adding random leaves keeps roots synced"
        );
    }

    /// Test: remove_leaf
    /// ------
    /// Add all leaves,
    /// then remove leaves
    #[test]
    fn test_remove_all() {
        let (mut merkle, mut off_chain_merkle) = setup();
        let mut rng = thread_rng();

        let num_leaves = 1 << MAX_DEPTH;
        add_random_leafs(&mut merkle, &mut off_chain_merkle, &mut rng, num_leaves);

        let mut inds: Vec<usize> = (0..num_leaves).collect();
        inds.shuffle(&mut rng);

        for idx in inds.into_iter() {
            let root = merkle.get();
            let (proof, path) = off_chain_merkle.get_proof_of_leaf(idx);
            merkle.remove(
                root,
                off_chain_merkle.get_node(idx),
                proof_to_slice(proof),
                path,
            );
            off_chain_merkle.remove_leaf(idx);
        }

        assert_eq!(merkle.get(), off_chain_merkle.root);
    }

    /// Test: add_leaf, remove_leaf
    /// ------
    /// Randomly insert & remove leaves into a half-full tree
    ///
    /// Description:
    /// Shuffle all the remaining leaves,
    ///      and either add to that leaf if it is empty
    ///      or remove leaf if it has values
    ///
    /// Note: we can only create proofs for up to MAX_SIZE indices at a time
    ///      before reconstructing our list of proofs
    ///
    /// Note: make sure indices are deduped, this cannot handle duplicate additions
    ///
    /// This test mimicks concurrent writes to the same merkle tree
    /// in the same block.
    #[test]
    fn test_mixed() {
        let (mut merkle, mut off_chain_merkle) = setup();
        let mut rng = thread_rng();

        add_random_leafs(
            &mut merkle,
            &mut off_chain_merkle,
            &mut rng,
            1 << (MAX_DEPTH - 1),
        );

        // Limited by MAX_SIZE
        let (proofs, _indices) =
            create_proofs_of_existence(&merkle, &off_chain_merkle, &mut rng, 1 << MAX_DEPTH);

        // Apply "concurrent" changes to on-chain merkle accumulator
        let root = merkle.get();
        for (i, leaf, proof, path) in proofs.iter() {
            if *leaf != [0; 32] {
                println!("Remove {}", i);
                merkle.remove(root, off_chain_merkle.get_node(*i), *proof, *path);
                off_chain_merkle.remove_leaf(*i);
            } else {
                println!("Add {}", i);
                let random_leaf = rng.gen::<Node>();
                merkle.add(root, random_leaf, *proof, *path);
                off_chain_merkle.add_leaf(random_leaf, *i);
            }
            assert_eq!(merkle.get(), off_chain_merkle.root);
        }
    }

    /// Currently failing, need some fancy on-chain instructions & storage to be able to dynamically handle this
    #[test]
    fn test_write_conflict() {
        let (mut merkle, mut off_chain_merkle) = setup();
        let mut rng = thread_rng();

        // Setup on-chain & off-chain trees with a random node at index 0
        add_random_leafs(&mut merkle, &mut off_chain_merkle, &mut rng, 10);

        let root = merkle.get();

        println!("Pre conflict active tree root: {:?}", off_chain_merkle.root);

        // Cause write-conflict by writing to same leaf using a proof for same root
        println!("Starting write conflict...");
        {
            let node_conflict = rng.gen::<Node>();
            off_chain_merkle.add_leaf(node_conflict, 0);
            let (proof_of_conflict, path_conflict) = off_chain_merkle.get_proof_of_leaf(0);
            println!("Writing on-chain merkle root");
            merkle.add(
                root,
                node_conflict,
                proof_to_slice(proof_of_conflict),
                path_conflict,
            );

            assert_eq!(
                merkle.get(),
                off_chain_merkle.root,
                "\n\nComparing roots after write-conflict. \nOn chain: {:?} \nOff chain {:?}\n",
                merkle.get(),
                off_chain_merkle.root,
            );
        }
    }

    #[inline]
    fn setup_new_with_root(rng: &mut ThreadRng) -> (MerkleAccumulator, MerkleTree) {
        let mut random_leaves = Vec::<Node>::new();
        for _ in 0..(1 << MAX_DEPTH) {
            random_leaves.push(rng.gen::<Node>());
        }

        let off_chain_merkle = MerkleTree::new(random_leaves);
        let merkle = MerkleAccumulator::new_with_root(off_chain_merkle.get());
        (merkle, off_chain_merkle)
    }

    /// Test: remove_leaf
    /// Removes all the leaves
    #[test]
    fn test_new_with_root_remove_all() {
        let mut rng = thread_rng();
        let (mut merkle, mut off_chain_merkle) = setup_new_with_root(&mut rng);

        assert_eq!(
            merkle.get(),
            off_chain_merkle.get(),
            "New with root works as expected"
        );

        let root = merkle.get();
        println!("root is: {:?}", root);

        let mut leaf_inds: Vec<usize> = (0..1 << MAX_DEPTH).collect();
        leaf_inds.shuffle(&mut rng);

        // Remove all leaves
        for (i, idx) in leaf_inds.iter().enumerate() {
            println!("removing leaf {}: {}", i, idx);
            let (proof_vec, path) = off_chain_merkle.get_proof_of_leaf(*idx);

            merkle.remove(
                off_chain_merkle.get(),
                off_chain_merkle.get_node(*idx),
                proof_to_slice(proof_vec),
                path,
            );
            off_chain_merkle.remove_leaf(*idx);

            assert_eq!(
                merkle.get(),
                off_chain_merkle.get(),
                "Removing node modifies root correctly"
            );
        }
    }

    /// Test: remove_leaf
    /// Removes all the leaves in batches of max_size
    #[test]
    fn test_new_with_root_remove_all_batched() {
        let mut rng = thread_rng();
        let (mut merkle, mut off_chain_merkle) = setup_new_with_root(&mut rng);

        assert_eq!(
            merkle.get(),
            off_chain_merkle.get(),
            "New with root works as expected"
        );

        let root = merkle.get();
        println!("root is: {:?}", root);

        let mut leaf_inds: Vec<usize> = (0..1 << MAX_DEPTH).collect();
        leaf_inds.shuffle(&mut rng);

        // Remove all leaves
        for (batch_idx, chunk) in leaf_inds.chunks(MAX_SIZE).enumerate() {
            println!("Batch index: {}", batch_idx);

            let root = off_chain_merkle.get();
            for (i, leaf_idx) in chunk.iter().enumerate() {
                println!("removing leaf {}: {}", i, leaf_idx);
                let (proof_vec, path) = off_chain_merkle.get_proof_of_leaf(*leaf_idx);

                merkle.remove(
                    root,
                    off_chain_merkle.get_node(*leaf_idx),
                    proof_to_slice(proof_vec),
                    path,
                );
            }

            for leaf_idx in chunk.iter() {
                off_chain_merkle.remove_leaf(*leaf_idx);
            }

            assert_eq!(
                merkle.get(),
                off_chain_merkle.get(),
                "Removing node modifies root correctly"
            );
        }
    }

    /// Text new with root mixed (SHOULD FAIL)
    /// ------
    /// Queue instructions to add and remove the same leaves within the same block
    /// 1. First removes the leaves
    /// 2. Then adds the same leaves back
    /// (within the same block)
    /// 
    /// Should fail to add the first leaf back because the proof is wrong 
    /// - `add` instruction assumes that the previous leaf value was 0s
    #[test]
    fn test_new_with_root_mixed_should_fail() {
        let mut rng = thread_rng();
        let (mut merkle, off_chain_merkle) = setup_new_with_root(&mut rng);

        // Test remove_leaf
        let mut leaf_inds: Vec<usize> = (0..1 << MAX_DEPTH).collect();
        leaf_inds.shuffle(&mut rng);
        let num_to_take = 1; 

        let removed_inds: Vec<usize> = leaf_inds.into_iter().take(num_to_take).collect();
        println!("Removing {} indices", removed_inds.len());

        let root = off_chain_merkle.get();
        println!("root is: {:?}", root);
        // - remove leaves
        for idx in removed_inds.iter().rev() {
            println!("removing leaf: {}", idx);
            let (proof_vec, path) = off_chain_merkle.get_proof_of_leaf(*idx);
            merkle.remove(
                root,
                off_chain_merkle.get_node(*idx),
                proof_to_slice(proof_vec),
                path,
            );
        }

        // - add leaves back
        for idx in removed_inds.iter() {
            println!("adding leaf back: {}", idx);
            let (proof_vec, path) = off_chain_merkle.get_proof_of_leaf(*idx);

            // First call here should fail
            merkle.add(
                root,
                off_chain_merkle.get_node(*idx),
                proof_to_slice(proof_vec),
                path,
            );
        }
    }
}
