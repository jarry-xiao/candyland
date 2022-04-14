pub mod merkle;
use crate::merkle::{empty_node, recompute, Node, EMPTY, MASK, MAX_DEPTH, MAX_SIZE, PADDING};
use solana_program::keccak::hashv;

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

    /// New with root requires path to right most leaf, proof, and index of right most leaf
    pub fn new_with_root(
        root: Node,
        right_most_leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> Self {
        let mut roots = [empty_node(MAX_DEPTH as u32); MAX_SIZE];
        roots[0] = root;
        let rightmost_proof = Path {
            proof,
            index: index + 1,
            leaf: right_most_leaf,
        };
        assert_eq!(root, recompute(right_most_leaf, &proof, index));
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

    /// Only used to initialize right most path for a completely empty tree
    #[inline(always)]
    fn initialize_tree(&mut self, leaf: Node, mut proof: [Node; MAX_DEPTH]) -> Option<Node> {
        let old_root = recompute(EMPTY, &proof, 0);
        if old_root == empty_node(MAX_DEPTH as u32) {
            self.update_and_apply_proof(EMPTY, leaf, &mut proof, 0, 0, false)
        } else {
            None
        }
    }

    /// Basic operation that always succeeds
    pub fn append(&mut self, mut node: Node) -> Option<Node> {
        if node == EMPTY {
            return None;
        }
        if self.rightmost_proof.index >= 1 << MAX_DEPTH {
            return None;
        }
        if self.rightmost_proof.index == 0 {
            return self.initialize_tree(node, self.rightmost_proof.proof);
        }
        let leaf = node.clone();
        let intersection = self.rightmost_proof.index.trailing_zeros() as usize;
        let mut change_list = [EMPTY; MAX_DEPTH];
        let mut intersection_node = self.rightmost_proof.leaf;

        // Compute proof to the appended node from empty nodes
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

        // Compute the where the new node intersects the main tree
        change_list[intersection] = node;
        let hash = hashv(&[&intersection_node, &node]);
        node.copy_from_slice(hash.as_ref());
        self.rightmost_proof.proof[intersection] = intersection_node;

        // Update the change list path up to the root
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
            prev_leaf: EMPTY,
            index: self.rightmost_proof.index,
        };
        self.rightmost_proof.index = self.rightmost_proof.index + 1;
        self.rightmost_proof.leaf = leaf;
        Some(node)
    }

    /// Convenience function for `set_leaf`
    /// On write conflict:
    /// Will append
    pub fn fill_empty_or_append(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> Option<Node> {
        self._find_and_update_leaf(current_root, EMPTY, leaf, proof, index, true)
    }

    /// Convenience function for `set_leaf`
    /// On write conflict:
    /// Will fail by returning None
    pub fn set_leaf_to_empty(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> Option<Node> {
        if index > self.rightmost_proof.index {
            return None;
        }
        self._find_and_update_leaf(current_root, leaf, EMPTY, proof, index, false)
    }

    /// On write conflict:
    /// Will fail by returning None
    pub fn set_leaf(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> Option<Node> {
        self._find_and_update_leaf(current_root, leaf, new_leaf, proof, index, false)
    }

    /// Internal function used to set leaf value & record changelog
    fn _find_and_update_leaf(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        mut proof: [Node; MAX_DEPTH],
        index: u32,
        append_on_conflict: bool,
    ) -> Option<Node> {
        for i in 0..self.buffer_size {
            let j = self.active_index.wrapping_sub(i) & MASK;
            if self.roots[j] != current_root {
                continue;
            }
            let old_root = recompute(leaf, &proof, index);
            if old_root == current_root && index > self.rightmost_proof.index && append_on_conflict
            {
                println!("RMP index: {}", self.rightmost_proof.index);
                println!("Leaf index: {}", index);
                return self.append(new_leaf);
            } else if old_root == current_root {
                return self.update_and_apply_proof(
                    leaf,
                    new_leaf,
                    &mut proof,
                    index,
                    j,
                    append_on_conflict,
                );
            } else {
                return None;
            }
        }
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
        append_on_conflict: bool,
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
            if leaf == EMPTY && append_on_conflict {
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
        } else {
            assert!(index == self.rightmost_proof.index);
            self.rightmost_proof.proof.copy_from_slice(&proof);
            self.rightmost_proof.index = index + 1;
            self.rightmost_proof.leaf = curr_leaf;
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
            let leaf = EMPTY;
            leaves.push(leaf);
        }
        // off-chain merkle tree
        let uc_merkley = MerkleTree::new(leaves);

        (merkle, uc_merkley)
    }

    /// Adds random leaves to on-chain & records off-chain
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
            merkle.fill_empty_or_append(
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
        let mut slice = [EMPTY; MAX_DEPTH];
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

    /// Test: fill_empty_or_append
    /// ------
    /// Basic unit test
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

    /// Test: append
    /// ------
    /// Note: we are not initializing on-chain merkle accumulator, we just start using it to track changes
    #[test]
    fn test_append() {
        let (mut merkle, mut off_chain_merkle) = setup();
        let mut rng = thread_rng();

        println!("Accumulator init root     : {:?}", merkle.get_root());
        println!("Off-chain merkle init root: {:?}", off_chain_merkle.root);

        for i in 0..1 << MAX_DEPTH {
            let leaf = rng.gen::<Node>();
            merkle.append(leaf);
            off_chain_merkle.add_leaf(leaf, i);
        }

        assert_eq!(merkle.get_root(), off_chain_merkle.root);
    }

    /// Test: fill_or_empty appends on conflict
    /// ------
    /// We are attempting to overwrite the same exact leaf in the same index
    /// within a block. Only the first fill will succeed, the others should
    /// be converted to appends
    #[test]
    fn test_append_on_conflict() {
        let (mut merkle, mut off_chain_merkle) = setup();
        let mut rng = thread_rng();

        println!("Accumulator init root     : {:?}", merkle.get_root());
        println!("Off-chain merkle init root: {:?}", off_chain_merkle.root);

        let indices = (0..1 << MAX_DEPTH).collect::<Vec<usize>>();
        for (i, chunk) in indices.chunks(MAX_SIZE).enumerate() {
            let mut leaves = vec![];
            for _ in chunk.iter() {
                let leaf = rng.gen::<Node>();
                leaves.push(leaf);
                let slot = i * MAX_SIZE;
                merkle.fill_empty_or_append(
                    off_chain_merkle.get_root(),
                    leaf,
                    proof_to_slice(off_chain_merkle.get_proof_of_leaf(slot)),
                    slot as u32,
                );
            }
            for (leaf_idx, j) in chunk.iter().enumerate() {
                off_chain_merkle.add_leaf(leaves[leaf_idx], *j);
            }
        }

        assert_eq!(merkle.get_root(), off_chain_merkle.root);
    }

    /// Test: set_leaf_to_empty
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
            merkle.set_leaf_to_empty(
                root,
                off_chain_merkle.get_node(idx),
                proof_to_slice(proof),
                idx as u32,
            );
            off_chain_merkle.remove_leaf(idx);
        }

        assert_eq!(merkle.get_root(), off_chain_merkle.root);
    }

    /// Test: fill_empty_or_append, set_leaf_to_emtpy
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
            if *leaf != EMPTY {
                println!("Remove {}", i);
                merkle.set_leaf_to_empty(root, off_chain_merkle.get_node(*i), *proof, *i as u32);
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
                merkle.fill_empty_or_append(root, random_leaf, *proof, j as u32);
                off_chain_merkle.add_leaf(random_leaf, j);
            }
            assert_eq!(merkle.get_root(), off_chain_merkle.root);
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

    /// Test: new with root set_leaf_to_empty
    /// --------
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
            merkle.set_leaf_to_empty(
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

    /// Test: batched removes for new_with_root
    /// --------
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

                merkle.set_leaf_to_empty(
                    root,
                    off_chain_merkle.get_node(*leaf_idx),
                    proof_to_slice(proof_vec),
                    *leaf_idx as u32,
                );
            }

            for leaf_idx in chunk.iter() {
                off_chain_merkle.remove_leaf(*leaf_idx);
            }

            assert_eq!(
                merkle.get_root(),
                off_chain_merkle.get_root(),
                "Root mismatch"
            );
        }
    }

    /// Test remove & add in the same block
    /// ----
    /// Emptying a leaf ==> decompressing an NFT
    /// Replacing a leaf ==> transferring an NFT within the tree
    #[test]
    fn test_new_with_root_remove_and_add_fails() {
        let mut rng = thread_rng();
        let (mut merkle, off_chain_merkle) = setup_new_with_root(&mut rng);

        let mut leaf_inds: Vec<usize> = (0..1 << MAX_DEPTH).collect();
        leaf_inds.shuffle(&mut rng);

        let num_to_take = 1;

        let replaced_inds: Vec<usize> = leaf_inds.into_iter().take(num_to_take).collect();
        println!("Removing {} indices", replaced_inds.len());

        let root = off_chain_merkle.get_root();

        // Decompress an NFT
        for idx in replaced_inds.iter().rev() {
            println!("Zero-ing leaf at index: {}", idx);
            let proof_vec = off_chain_merkle.get_proof_of_leaf(*idx);
            let result = merkle.set_leaf_to_empty(
                root,
                off_chain_merkle.get_node(*idx),
                proof_to_slice(proof_vec),
                *idx as u32,
            );
            assert!(!result.is_none());
        }

        // Attempting to transfer ownership (replacing leaf hash)
        // within same block should fail
        for idx in replaced_inds.iter().rev() {
            println!("One-ing leaf at index: {}", idx);
            let proof_vec = off_chain_merkle.get_proof_of_leaf(*idx);
            let result = merkle.set_leaf(
                root,
                off_chain_merkle.get_node(*idx),
                [1; 32],
                proof_to_slice(proof_vec),
                *idx as u32,
            );
            assert!(result.is_none());
        }
    }

    /// Test multiple replaces within same block to same index
    /// ---
    /// Only the first replace should go through
    #[test]
    fn test_new_with_root_replace_bunch() {
        let mut rng = thread_rng();
        let (mut merkle, mut off_chain_merkle) = setup_new_with_root(&mut rng);

        let idx_to_replace = rng.gen_range(0, 1 << MAX_DEPTH);
        let proof_vec = off_chain_merkle.get_proof_of_leaf(idx_to_replace);
        let proof_slice = proof_to_slice(proof_vec);
        let root = off_chain_merkle.get_root();
        let start_node = rng.gen::<Node>();
        let mut last_node = start_node.clone();

        // replace same index with random #s
        for _ in 0..MAX_SIZE {
            println!("Setting leaf to value: {:?}", last_node);
            merkle.set_leaf(
                root,
                off_chain_merkle.get_node(idx_to_replace),
                last_node,
                proof_slice,
                idx_to_replace as u32,
            );
            last_node = rng.gen::<Node>();
        }

        off_chain_merkle.add_leaf(start_node, idx_to_replace);

        assert_eq!(merkle.get_root(), off_chain_merkle.get_root(),);
    }

    /// Text new with root mixed
    /// ------
    /// Queue instructions to add and remove the same leaves within the same block
    /// 1. First removes the leaves
    /// 2. Then adds the same leaves back
    /// (within the same block)
    ///
    #[test]
    fn test_new_with_root_mixed() {
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
            merkle.set_leaf_to_empty(
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
            merkle.fill_empty_or_append(
                root,
                off_chain_merkle.get_node(*idx),
                proof_to_slice(proof_vec),
                *idx as u32,
            );
        }
    }
}
