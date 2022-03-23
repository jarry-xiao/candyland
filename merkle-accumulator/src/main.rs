use solana_program::keccak::hashv;
mod merkle;
use crate::merkle::{empty_node, recompute, Node, MASK, MAX_DEPTH, MAX_SIZE, PADDING};

#[derive(Default, Copy, Clone, PartialEq)]
/// Stores proof for a given Merkle root update
pub struct ChangeLog {
    /// Nodes of off-chain merkle tree
    path: [Node; MAX_DEPTH],
    prev_leaf: Node,
    curr_leaf: Node,
    /// Bitmap of node parity (used when hashing)
    index: u32,
}

#[derive(Default, Copy, Clone, PartialEq)]
/// Stores proof for a given Merkle root update
pub struct Path {
    proof: [Node; MAX_DEPTH],
    leaf: Node,
    index: u32,
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
    rightmost_proof: Path,
}

impl MerkleAccumulator {
    pub fn new() -> Self {
        let mut rightmost_proof = Path::default();
        for (i, node) in rightmost_proof.proof.iter_mut().enumerate() {
            *node = empty_node(i as u32);
        }
        Self {
            roots: [empty_node(MAX_DEPTH as u32); MAX_SIZE],
            change_logs: [ChangeLog::default(); MAX_SIZE],
            active_index: 0,
            buffer_size: 1,
            rightmost_proof,
        }
    }

    pub fn new_with_root(root: Node, leaf: Node, proof: [Node; MAX_DEPTH], index: u32) -> Self {
        let mut roots = [empty_node(MAX_DEPTH as u32); MAX_SIZE];
        roots[0] = root;
        let rightmost_proof = Path {
            proof,
            index: index + 1,
            leaf,
        };
        assert_eq!(root, recompute(leaf, &proof, index));
        Self {
            roots,
            change_logs: [ChangeLog::default(); MAX_SIZE],
            active_index: 0,
            buffer_size: 1,
            rightmost_proof,
        }
    }

    /// Returns on-chain root
    pub fn get_root(&self) -> Node {
        self.roots[self.active_index]
    }

    pub fn append(&mut self, mut node: Node) -> Option<Node> {
        if node == [0; 32] {
            return Some(node);
        }
        if self.rightmost_proof.index >= 1 << MAX_DEPTH {
            return None;
        }
        if self.rightmost_proof.index == 0 {
            return self.initialize_tree(node, self.rightmost_proof.proof);
        }
        let leaf = node.clone();
        let intersection = self.rightmost_proof.index.trailing_zeros() as usize;
        let mut change_list = [[0; 32]; MAX_DEPTH];
        let mut intersection_node = self.rightmost_proof.leaf;

        for i in 0..intersection {
            change_list[i] = node;
            let hash = hashv(&[&node, &empty_node(i as u32)]);
            node.copy_from_slice(hash.as_ref());
            let rightmost_hash = if ((self.rightmost_proof.index - 1) >> i) & 1 == 1 {
                hashv(&[&self.rightmost_proof.proof[i], &intersection_node])
            } else {
                hashv(&[&intersection_node, &self.rightmost_proof.proof[i]])
            };
            intersection_node.copy_from_slice(rightmost_hash.as_ref());
            self.rightmost_proof.proof[i] = empty_node(i as u32);
        }
        change_list[intersection] = node;
        let hash = hashv(&[&intersection_node, &node]);
        node.copy_from_slice(hash.as_ref());
        self.rightmost_proof.proof[intersection] = intersection_node;
        for i in intersection + 1..MAX_DEPTH {
            change_list[i] = node;
            let hash = if (self.rightmost_proof.index >> i) & 1 == 1 {
                hashv(&[&self.rightmost_proof.proof[i], &node])
            } else {
                hashv(&[&node, &self.rightmost_proof.proof[i]])
            };
            node.copy_from_slice(hash.as_ref());
        }
        self.increment_active_index();
        self.roots[self.active_index] = node;
        self.change_logs[self.active_index] = ChangeLog {
            path: change_list,
            curr_leaf: leaf,
            prev_leaf: [0; 32],
            index: self.rightmost_proof.index,
        };
        self.rightmost_proof.index = self.rightmost_proof.index + 1;
        self.rightmost_proof.leaf = leaf;
        Some(node)
    }

    fn initialize_tree(&mut self, leaf: Node, mut proof: [Node; MAX_DEPTH]) -> Option<Node> {
        let old_root = recompute([0; 32], &proof, 0);
        if old_root == empty_node(MAX_DEPTH as u32) {
            self.update_and_apply_proof([0; 32], leaf, &mut proof, 0, 0)
        } else {
            println!("Bad proof");
            None
        }
    }

    pub fn add(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> Option<Node> {
        let root = if self.buffer_size == 0 {
            self.initialize_tree(leaf, proof)
        } else {
            self.replace(current_root, [0; 32], leaf, proof, index)
        };
        root
    }

    pub fn remove(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> Option<Node> {
        if index > self.rightmost_proof.index {
            return None;
        }
        self.replace(current_root, leaf, [0; 32], proof, index)
    }

    pub fn replace(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        mut proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> Option<Node> {
        for i in 0..self.buffer_size {
            let j = self.active_index.wrapping_sub(i) & MASK;
            if self.roots[j] != current_root {
                continue;
            }
            let old_root = recompute(leaf, &proof, index);
            if old_root == current_root && index > self.rightmost_proof.index {
                return self.append(new_leaf);
            } else if old_root == current_root {
                return self.update_and_apply_proof(leaf, new_leaf, &mut proof, index, j);
            } else {
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
        new_leaf: Node,
        proof: &mut [Node; MAX_DEPTH],
        index: u32,
        mut j: usize,
    ) -> Option<Node> {
        let mut updated_leaf = leaf;
        while j != self.active_index {
            // Implement circular index addition
            j += 1;
            j &= MASK;
            if index != self.change_logs[j].index {
                let common_path_len =
                    ((index ^ self.change_logs[j].index) << PADDING).leading_zeros() as usize;
                let critbit_index = (MAX_DEPTH - 1) - common_path_len;
                proof[critbit_index] = self.change_logs[j].path[critbit_index];
            } else {
                updated_leaf = self.change_logs[j].curr_leaf;
            }
        }
        let old_root = recompute(updated_leaf, proof, index);
        assert!(old_root == self.get_root());
        if updated_leaf != leaf {
            if leaf == [0; 32] {
                println!("Value is updated, appending to tree");
                return self.append(new_leaf);
            } else {
                return None;
            }
        }
        self.increment_active_index();
        let new_root = self.apply_changes(leaf, new_leaf, proof, index);
        self.roots[self.active_index] = new_root;
        Some(new_root)
    }

    fn increment_active_index(&mut self) {
        self.active_index += 1;
        self.active_index &= MASK;
        if self.buffer_size < MAX_SIZE {
            self.buffer_size += 1;
        }
    }

    /// Creates a new root from a proof that is valid for the root at `self.active_index`
    fn apply_changes(
        &mut self,
        prev_leaf: Node,
        mut start: Node,
        proof: &[Node],
        index: u32,
    ) -> Node {
        let curr_leaf = start.clone();
        let change_log = &mut self.change_logs[self.active_index];
        change_log.path[0] = start;
        for (ix, s) in proof.iter().enumerate() {
            if index >> ix & 1 == 0 {
                let res = hashv(&[&start, s.as_ref()]);
                start.copy_from_slice(res.as_ref());
            } else {
                let res = hashv(&[s.as_ref(), &start]);
                start.copy_from_slice(res.as_ref());
            }
            if ix < MAX_DEPTH - 1 {
                change_log.path[ix + 1] = start;
            }
        }
        change_log.index = index;
        change_log.prev_leaf = prev_leaf;
        change_log.curr_leaf = curr_leaf;
        if index < self.rightmost_proof.index as u32 {
            if index != self.rightmost_proof.index - 1 {
                let common_path_len = ((index ^ (self.rightmost_proof.index - 1) as u32) << PADDING)
                    .leading_zeros() as usize;
                let critbit_index = (MAX_DEPTH - 1) - common_path_len;
                self.rightmost_proof.proof[critbit_index] = change_log.path[critbit_index];
            }
        } else if index == self.rightmost_proof.index {
            self.rightmost_proof.proof.copy_from_slice(&proof);
            self.rightmost_proof.index = index + 1;
            self.rightmost_proof.leaf = curr_leaf;
        } else {
            unreachable!();
        }
        start
    }
}

fn main() {}

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
        println!("Starting root {:?}", off_chain_merkle.get_root());
        for i in 0..num {
            let leaf = rng.gen::<Node>();
            let proof = off_chain_merkle.get_proof_of_leaf(i);
            merkle.add(
                off_chain_merkle.get_root(),
                leaf,
                proof_to_slice(proof),
                i as u32,
            );
            off_chain_merkle.add_leaf(leaf, i);
        }

        assert_eq!(
            merkle.get_root(),
            off_chain_merkle.get_root(),
            "Adding random leaves keeps roots synced"
        );
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
    ) -> (Vec<(usize, Node, [Node; MAX_DEPTH])>, Vec<usize>) {
        let mut inds: Vec<usize> = (0..num_leaves).collect();
        inds.shuffle(rng);
        let mut proofs = vec![];
        let mut indices = vec![];

        for i in inds.into_iter().take(MAX_SIZE - 1) {
            let proof = off_chain_merkle.get_proof_of_leaf(i);

            // Make on-chain readable proof

            proofs.push((i, off_chain_merkle.get_node(i), proof_to_slice(proof)));
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

        println!("Accumulator init root     : {:?}", merkle.get_root());
        println!("Off-chain merkle init root: {:?}", off_chain_merkle.root);

        add_random_leafs(&mut merkle, &mut off_chain_merkle, &mut rng, 1 << MAX_DEPTH);

        assert_eq!(
            merkle.get_root(),
            off_chain_merkle.root,
            "Adding random leaves keeps roots synced"
        );
    }

    // Test: add_leaf
    // ------
    // Note: we are not initializing on-chain merkle accumulator, we just start using it to track changes
    // Off-chain: replace 1st half of leaves with random values
    // On-chain: record updates to the root
    #[test]
    fn test_append() {
        let (mut merkle, mut off_chain_merkle) = setup();
        let mut rng = thread_rng();

        println!("Accumulator init root     : {:?}", merkle.get_root());
        println!("Off-chain merkle init root: {:?}", off_chain_merkle.root);

        for i in 0..128 {
            let leaf = rng.gen::<Node>();
            merkle.append(leaf);
            off_chain_merkle.add_leaf(leaf, i);
        }

        assert_eq!(merkle.get_root(), off_chain_merkle.root);
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
            let root = merkle.get_root();
            let proof = off_chain_merkle.get_proof_of_leaf(idx);
            merkle.remove(
                root,
                off_chain_merkle.get_node(idx),
                proof_to_slice(proof),
                idx as u32,
            );
            off_chain_merkle.remove_leaf(idx);
        }

        assert_eq!(merkle.get_root(), off_chain_merkle.root);
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
        let root = merkle.get_root();
        let mut appended_indices = vec![];
        for (i, leaf, proof) in proofs.iter() {
            if *leaf != [0; 32] {
                println!("Remove {}", i);
                merkle.remove(root, off_chain_merkle.get_node(*i), *proof, *i as u32);
                off_chain_merkle.remove_leaf(*i);
                if appended_indices.contains(i) {
                    appended_indices.retain(|&x| x != *i);
                }
            } else {
                println!("Add {}", i);
                let random_leaf = rng.gen::<Node>();
                let j = if *i >= merkle.rightmost_proof.index as usize {
                    let j = merkle.rightmost_proof.index as usize;
                    appended_indices.push(j);
                    j
                } else {
                    if appended_indices.contains(i) {
                        let j = merkle.rightmost_proof.index as usize;
                        appended_indices.push(j);
                        j
                    } else {
                        *i
                    }
                };
                merkle.add(root, random_leaf, *proof, j as u32);
                off_chain_merkle.add_leaf(random_leaf, j);
            }
            assert_eq!(merkle.get_root(), off_chain_merkle.root);
        }
    }

    /// Currently failing, need some fancy on-chain instructions & storage to be able to dynamically handle this
    #[test]
    fn test_write_conflict_should_fail() {
        let (mut merkle, mut off_chain_merkle) = setup();
        let mut rng = thread_rng();

        // Setup on-chain & off-chain trees with a random node at index 0
        let proof_of_conflict = off_chain_merkle.get_proof_of_leaf(0);
        let root = off_chain_merkle.get_root();
        println!("Starting root (conflict) {:?}", off_chain_merkle.get_root());
        assert_eq!(off_chain_merkle.get_root(), merkle.get_root());

        add_random_leafs(&mut merkle, &mut off_chain_merkle, &mut rng, 10);

        println!("Pre conflict active tree root: {:?}", off_chain_merkle.root);

        // Cause write-conflict by writing to same leaf using a proof for same root
        println!("Starting write conflict...");
        {
            let node_conflict = rng.gen::<Node>();
            off_chain_merkle.add_leaf(node_conflict, 10);
            println!("Writing on-chain merkle root");
            for x in proof_of_conflict.iter() {
                println!("    {:?}", x);
            }
            merkle.add(root, node_conflict, proof_to_slice(proof_of_conflict), 0);

            assert_eq!(
                merkle.get_root(),
                off_chain_merkle.root,
                "\n\nComparing roots after write-conflict. \nOn chain: {:?} \nOff chain {:?}\n",
                merkle.get_root(),
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

        let i = (1 << MAX_DEPTH) - 1;
        let leaf = random_leaves[i].clone();
        let off_chain_merkle = MerkleTree::new(random_leaves);
        let proof = off_chain_merkle.get_proof_of_leaf(i);

        let merkle = MerkleAccumulator::new_with_root(
            off_chain_merkle.get_root(),
            leaf,
            proof.try_into().unwrap(),
            i as u32,
        );
        (merkle, off_chain_merkle)
    }

    /// Test: remove_leaf
    /// Removes all the leaves
    #[test]
    fn test_new_with_root_remove_all() {
        let mut rng = thread_rng();
        let (mut merkle, mut off_chain_merkle) = setup_new_with_root(&mut rng);

        assert_eq!(
            merkle.get_root(),
            off_chain_merkle.get_root(),
            "New with root works as expected"
        );

        let root = merkle.get_root();
        println!("root is: {:?}", root);

        let mut leaf_inds: Vec<usize> = (0..1 << MAX_DEPTH).collect();
        leaf_inds.shuffle(&mut rng);

        // Remove all leaves
        for (i, idx) in leaf_inds.iter().enumerate() {
            let proof_vec = off_chain_merkle.get_proof_of_leaf(*idx);

            println!("off-chain");
            for x in proof_vec.iter() {
                println!("  {:?}", x);
            }
            merkle.remove(
                off_chain_merkle.get_root(),
                off_chain_merkle.get_node(*idx),
                proof_to_slice(proof_vec),
                *idx as u32,
            );
            off_chain_merkle.remove_leaf(*idx);

            assert_eq!(
                merkle.get_root(),
                off_chain_merkle.get_root(),
                "Removing node modifies root correctly {} {}",
                i,
                idx,
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
            merkle.get_root(),
            off_chain_merkle.get_root(),
            "New with root works as expected"
        );

        let root = merkle.get_root();
        println!("root is: {:?}", root);

        let mut leaf_inds: Vec<usize> = (0..1 << MAX_DEPTH).collect();
        leaf_inds.shuffle(&mut rng);

        // Remove all leaves
        for (batch_idx, chunk) in leaf_inds.chunks(MAX_SIZE).enumerate() {
            println!("Batch index: {}", batch_idx);

            let root = off_chain_merkle.get_root();
            for (i, leaf_idx) in chunk.iter().enumerate() {
                println!("removing leaf {}: {}", i, leaf_idx);
                let proof_vec = off_chain_merkle.get_proof_of_leaf(*leaf_idx);

                merkle.remove(
                    root,
                    off_chain_merkle.get_node(*leaf_idx),
                    proof_to_slice(proof_vec),
                    i as u32,
                );
            }

            for leaf_idx in chunk.iter() {
                off_chain_merkle.remove_leaf(*leaf_idx);
            }

            assert_eq!(
                merkle.get_root(),
                off_chain_merkle.get_root(),
                "Removing node modifies root correctly"
            );
        }
    }

    /// Test new with root replace same
    /// ----
    /// Replace the same leaves within the same block
    /// This should work... but might cause unexpected behavior
    #[test]
    fn test_new_with_root_replace_same() {
        let mut rng = thread_rng();
        let (mut merkle, mut off_chain_merkle) = setup_new_with_root(&mut rng);

        let mut leaf_inds: Vec<usize> = (0..1 << MAX_DEPTH).collect();
        leaf_inds.shuffle(&mut rng);

        // Replace (max size / 2) leaves 2x
        // this is the exact # of items that can be updated before off chain tree has to sync
        let num_to_take = MAX_SIZE >> 1;

        let replaced_inds: Vec<usize> = leaf_inds.into_iter().take(num_to_take).collect();
        println!("Removing {} indices", replaced_inds.len());

        let root = off_chain_merkle.get_root();
        println!("root is: {:?}", root);

        // - replace same leaves with 0s
        for idx in replaced_inds.iter().rev() {
            println!("Zero-ing leaf at index: {}", idx);
            let proof_vec = off_chain_merkle.get_proof_of_leaf(*idx);
            merkle.replace(
                root,
                off_chain_merkle.get_node(*idx),
                [0; 32],
                proof_to_slice(proof_vec),
                *idx as u32,
            );
        }

        // - replace same leaves with 1s
        for idx in replaced_inds.iter().rev() {
            println!("One-ing leaf at index: {}", idx);
            let proof_vec = off_chain_merkle.get_proof_of_leaf(*idx);
            merkle.replace(
                root,
                off_chain_merkle.get_node(*idx),
                [1; 32],
                proof_to_slice(proof_vec),
                *idx as u32,
            );
        }

        // Update off-chain merkle tree to match
        for idx in replaced_inds.iter() {
            off_chain_merkle.add_leaf([1; 32], *idx);
        }

        assert_eq!(
            merkle.get_root(),
            off_chain_merkle.get_root(),
            "Removing node modifies root correctly"
        );
    }

    /// Test multiple replaces within same block to same index
    /// ---
    /// All replaces should work, and only the last one should be reflected
    #[test]
    fn test_new_with_root_replace_bunch() {
        let mut rng = thread_rng();
        let (mut merkle, mut off_chain_merkle) = setup_new_with_root(&mut rng);

        let idx_to_replace = rng.gen_range(0, 1 << MAX_DEPTH);
        let proof_vec = off_chain_merkle.get_proof_of_leaf(idx_to_replace);
        let proof_slice = proof_to_slice(proof_vec);
        let root = off_chain_merkle.get_root();
        let mut last_node = [0; 32];

        // replace same index with random #s
        for _ in 0..MAX_SIZE {
            last_node = rng.gen::<Node>();
            println!("Setting leaf to value: {:?}", last_node);
            merkle.replace(
                root,
                off_chain_merkle.get_node(idx_to_replace),
                last_node,
                proof_slice,
                idx_to_replace as u32,
            );
        }

        off_chain_merkle.add_leaf(last_node, idx_to_replace);

        assert_eq!(
            merkle.get_root(),
            off_chain_merkle.get_root(),
            "Removing node modifies root correctly"
        );
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

        let mut leaf_inds: Vec<usize> = (0..1 << MAX_DEPTH).collect();
        leaf_inds.shuffle(&mut rng);
        let num_to_take = 1;

        let removed_inds: Vec<usize> = leaf_inds.into_iter().take(num_to_take).collect();
        println!("Removing {} indices", removed_inds.len());

        let root = off_chain_merkle.get_root();
        println!("root is: {:?}", root);
        // - remove leaves
        for idx in removed_inds.iter().rev() {
            println!("removing leaf: {}", idx);
            let proof_vec = off_chain_merkle.get_proof_of_leaf(*idx);
            merkle.remove(
                root,
                off_chain_merkle.get_node(*idx),
                proof_to_slice(proof_vec),
                *idx as u32,
            );
        }

        // - add leaves back
        for idx in removed_inds.iter() {
            println!("adding leaf back: {}", idx);
            let proof_vec = off_chain_merkle.get_proof_of_leaf(*idx);

            // First call here should fail
            merkle.add(
                root,
                off_chain_merkle.get_node(*idx),
                proof_to_slice(proof_vec),
                *idx as u32,
            );
        }
    }
}
