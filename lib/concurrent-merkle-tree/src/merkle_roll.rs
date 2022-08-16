use crate::{
    error::CMTError,
    state::{ChangeLog, Node, Path, EMPTY},
    utils::{empty_node, empty_node_cached, fill_in_proof, hash_to_parent, recompute},
};
use bytemuck::{Pod, Zeroable};
pub(crate) use log_compute;
pub(crate) use solana_logging;

#[cfg(feature = "sol-log")]
use solana_program::{log::sol_log_compute_units, msg};

#[inline(always)]
fn check_bounds(max_depth: usize, max_buffer_size: usize) {
    assert!(max_depth < 31);
    // This will return true if MAX_BUFFER_SIZE is a power of 2 or if it is 0
    assert!(max_buffer_size & (max_buffer_size - 1) == 0);
}
/// Tracks updates to off-chain Merkle tree
///
/// Allows for concurrent writes to same merkle tree so long as proof
/// was generated at most MAX_SIZE updates since the tx was submitted
#[derive(Copy, Clone)]
pub struct MerkleRoll<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> {
    pub sequence_number: u64,
    /// Index of most recent root & changes
    pub active_index: u64,
    /// Number of active changes we are tracking
    pub buffer_size: u64,
    /// Proof for respective root
    pub change_logs: [ChangeLog<MAX_DEPTH>; MAX_BUFFER_SIZE],
    pub rightmost_proof: Path<MAX_DEPTH>,
}

unsafe impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Zeroable
    for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}
unsafe impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Pod
    for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}

impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE> {
    pub fn new() -> Self {
        Self {
            sequence_number: 0,
            active_index: 0,
            buffer_size: 0,
            change_logs: [ChangeLog::<MAX_DEPTH>::default(); MAX_BUFFER_SIZE],
            rightmost_proof: Path::<MAX_DEPTH>::default(),
        }
    }

    pub fn initialize(&mut self) -> Result<Node, CMTError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        let mut rightmost_proof = Path::default();
        let mut empty_node_cache = Box::new([Node::default(); MAX_DEPTH]);
        for (i, node) in rightmost_proof.proof.iter_mut().enumerate() {
            *node = empty_node_cached::<MAX_DEPTH>(i as u32, &mut empty_node_cache);
        }
        let mut path = [Node::default(); MAX_DEPTH];
        for (i, node) in path.iter_mut().enumerate() {
            *node = empty_node_cached::<MAX_DEPTH>(i as u32, &mut empty_node_cache);
        }
        self.change_logs[0].root = empty_node(MAX_DEPTH as u32);
        self.change_logs[0].path = path;
        self.sequence_number = 0;
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        Ok(self.change_logs[0].root)
    }

    pub fn initialize_with_root(
        &mut self,
        root: Node,
        rightmost_leaf: Node,
        proof_vec: &Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
        proof.copy_from_slice(&proof_vec[..]);
        let rightmost_proof = Path {
            proof,
            index: index + 1,
            leaf: rightmost_leaf,
            _padding: 0,
        };
        self.change_logs[0].root = root;
        self.sequence_number = 1;
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        assert_eq!(root, recompute(rightmost_leaf, &proof, index,));
        Ok(root)
    }

    pub fn get_change_log(&self) -> Box<ChangeLog<MAX_DEPTH>> {
        Box::new(self.change_logs[self.active_index as usize])
    }

    pub fn prove_leaf(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof_vec: &Vec<Node>,
        leaf_index: u32,
    ) -> Result<Node, CMTError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        if leaf_index > self.rightmost_proof.index {
            solana_logging!(
                "Received an index larger than the rightmost index {} > {}",
                leaf_index,
                self.rightmost_proof.index
            );
            return Err(CMTError::LeafIndexOutOfBounds);
        } else {
            let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
            fill_in_proof::<MAX_DEPTH>(proof_vec, &mut proof);
            let valid_root =
                self.check_valid_leaf(current_root, leaf, &mut proof, leaf_index, true)?;
            if !valid_root {
                solana_logging!("Proof failed to verify");
                return Err(CMTError::InvalidProof);
            }
            Ok(Node::default())
        }
    }

    /// Only used to initialize right most path for a completely empty tree
    #[inline(always)]
    fn initialize_tree_from_append(
        &mut self,
        leaf: Node,
        mut proof: [Node; MAX_DEPTH],
    ) -> Result<Node, CMTError> {
        let old_root = recompute(EMPTY, &proof, 0);
        if old_root == empty_node(MAX_DEPTH as u32) {
            self.try_apply_proof(old_root, EMPTY, leaf, &mut proof, 0, false)
        } else {
            return Err(CMTError::TreeAlreadyInitialized);
        }
    }

    fn update_state_from_append(
        &mut self,
        root: Node,
        change_list: [Node; MAX_DEPTH],
        rmp_index: u32,
        rmp_leaf: Node
    ) -> Result<(), CMTError> {
        self.update_internal_counters();
        self.change_logs[self.active_index as usize] =
            ChangeLog::<MAX_DEPTH>::new(root, change_list, rmp_index);
        self.rightmost_proof.index = rmp_index + 1;
        self.rightmost_proof.leaf = rmp_leaf;
        Ok(())
    }

    /// Append leaf to tree
    pub fn append(&mut self, mut node: Node) -> Result<Node, CMTError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        if node == EMPTY {
            return Err(CMTError::CannotAppendEmptyNode);
        }
        if self.rightmost_proof.index >= 1 << MAX_DEPTH {
            return Err(CMTError::TreeFull);
        }
        if self.rightmost_proof.index == 0 {
            return self.initialize_tree_from_append(node, self.rightmost_proof.proof);
        }
        let leaf = node.clone();
        let intersection = self.rightmost_proof.index.trailing_zeros() as usize;
        let mut change_list = [EMPTY; MAX_DEPTH];
        let mut intersection_node = self.rightmost_proof.leaf;
        let mut empty_node_cache = Box::new([Node::default(); MAX_DEPTH]);

        for i in 0..MAX_DEPTH {
            change_list[i] = node;
            if i < intersection {
                // Compute proof to the appended node from empty nodes
                let sibling = empty_node_cached::<MAX_DEPTH>(i as u32, &mut empty_node_cache);
                hash_to_parent(
                    &mut intersection_node,
                    &self.rightmost_proof.proof[i],
                    ((self.rightmost_proof.index - 1) >> i) & 1 == 0,
                );
                hash_to_parent(&mut node, &sibling, true);
                self.rightmost_proof.proof[i] = sibling;
            } else if i == intersection {
                // Compute the where the new node intersects the main tree
                hash_to_parent(&mut node, &intersection_node, false);
                self.rightmost_proof.proof[intersection] = intersection_node;
            } else {
                // Update the change list path up to the root
                hash_to_parent(
                    &mut node,
                    &self.rightmost_proof.proof[i],
                    ((self.rightmost_proof.index - 1) >> i) & 1 == 0,
                );
            }
        }
        self.update_state_from_append(node, change_list, self.rightmost_proof.index, leaf)?;
        Ok(node)
    }

    fn initialize_tree_from_subtree_append(
        &mut self,
        subtree_root: Node,
        subtree_rightmost_leaf: Node,
        subtree_rightmost_index: u32,
        subtree_rightmost_proof: Vec<Node>
    ) -> Result<Node, CMTError> {

        let leaf = subtree_rightmost_leaf.clone();
        let mut change_list = [EMPTY; MAX_DEPTH];

        // This will be mutated into the new root after the append by gradually hashing this node with the RMP to the subtree, then the critical node, and then the rest of the RMP to this tree.
        let mut node = subtree_rightmost_leaf;
        for i in 0..MAX_DEPTH {
            change_list[i] = node;
            if i < subtree_rightmost_proof.len() {
                // Hash up to subtree_root using subtree_rmp, to create accurate change_list
                hash_to_parent(
                    &mut node,
                    &subtree_rightmost_proof[i],
                    ((subtree_rightmost_index - 1) >> i) & 1 == 0,
                );
                self.rightmost_proof.proof[i] = subtree_rightmost_proof[i];
            } else {
                // Compute where the new node intersects the main tree
                if i == subtree_rightmost_proof.len() {
                    assert!(node == subtree_root);
                }

                hash_to_parent(&mut node, &self.rightmost_proof.proof[i], true);
                // No need to update the RMP anymore
            }
        }
        self.update_state_from_append(node, change_list, self.rightmost_proof.index + subtree_rightmost_index - 1, leaf)?;
        Ok(node)
    }

    /// Append subtree to current tree
    pub fn append_subtree(
        &mut self,
        subtree_root: Node,
        subtree_rightmost_leaf: Node,
        subtree_rightmost_index: u32,
        subtree_rightmost_proof: Vec<Node>,
    ) -> Result<Node, CMTError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        if self.rightmost_proof.index >= 1 << MAX_DEPTH {
            return Err(CMTError::TreeFull);
        }

        // Confirm that subtree_rightmost_proof is valid
        if recompute(
            subtree_rightmost_leaf,
            &subtree_rightmost_proof[..],
            subtree_rightmost_index - 1,
        ) != subtree_root
        {
            return Err(CMTError::InvalidProof);
        }

        let intersection = self.rightmost_proof.index.trailing_zeros() as usize;

        if self.rightmost_proof.index == 0 {
            // @dev: If the tree has only empty leaves, then there are many sizes of tree which can be appended, but they cannot be larger than the tree itself
            if subtree_rightmost_proof.len() >= MAX_DEPTH {
                return Err(CMTError::SubtreeInvalidSize);
            }
            return self.initialize_tree_from_subtree_append(subtree_root, subtree_rightmost_leaf, subtree_rightmost_index, subtree_rightmost_proof);
        } else {
            // @dev: At any given time (other than initialization), there is only one valid size of subtree that can be appended
            if subtree_rightmost_proof.len() != intersection {
                return Err(CMTError::SubtreeInvalidSize);
            }
        }

        let leaf = subtree_rightmost_leaf.clone();
        let mut change_list = [EMPTY; MAX_DEPTH];
        let mut intersection_node = self.rightmost_proof.leaf;

        // This will be mutated into the new root after the append by gradually hashing this node with the RMP to the subtree, then the critical node, and then the rest of the RMP to this tree.
        let mut node = subtree_rightmost_leaf;

        for i in 0..MAX_DEPTH {
            change_list[i] = node;
            if i < intersection {
                // Compute proof to the appended node from empty nodes
                hash_to_parent(
                    &mut intersection_node,
                    &self.rightmost_proof.proof[i],
                    ((self.rightmost_proof.index - 1) >> i) & 1 == 0,
                );
                hash_to_parent(
                    &mut node,
                    &subtree_rightmost_proof[i],
                    ((subtree_rightmost_index - 1) >> i) & 1 == 0,
                );
                self.rightmost_proof.proof[i] = subtree_rightmost_proof[i];
            } else if i == intersection {
                // Compute where the new node intersects the main tree
                assert!(node == subtree_root);
                hash_to_parent(&mut node, &intersection_node, false);
                self.rightmost_proof.proof[intersection] = intersection_node;
            } else {
                // Update the change list path up to the root
                hash_to_parent(
                    &mut node,
                    &self.rightmost_proof.proof[i],
                    ((self.rightmost_proof.index - 1) >> i) & 1 == 0,
                );
            }
        }
        self.update_state_from_append(node, change_list, self.rightmost_proof.index + subtree_rightmost_index - 1, leaf)?;
        Ok(node)
    }

    /// Convenience function for `set_leaf`
    /// On write conflict:
    /// Will append
    pub fn fill_empty_or_append(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof_vec: &Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
        fill_in_proof::<MAX_DEPTH>(&proof_vec, &mut proof);
        log_compute!();
        let root = match self.try_apply_proof(current_root, EMPTY, leaf, &mut proof, index, false) {
            Ok(new_root) => Ok(new_root),
            Err(error) => match error {
                CMTError::LeafContentsModified => self.append(leaf),
                _ => Err(error),
            },
        };
        log_compute!();
        root
    }

    /// On write conflict:
    /// Will fail by returning None
    pub fn set_leaf(
        &mut self,
        current_root: Node,
        previous_leaf: Node,
        new_leaf: Node,
        proof_vec: &Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        if index > self.rightmost_proof.index {
            return Err(CMTError::LeafIndexOutOfBounds);
        } else {
            let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
            fill_in_proof::<MAX_DEPTH>(&proof_vec, &mut proof);
            log_compute!();
            let root = self.try_apply_proof(
                current_root,
                previous_leaf,
                new_leaf,
                &mut proof,
                index,
                true,
            );
            log_compute!();
            root
        }
    }

    /// Modifies the `proof` for leaf at `leaf_index`
    /// in place by fast-forwarding the given `proof` through the
    /// `changelog`s, starting at index `changelog_buffer_index`
    /// Returns false if the leaf was updated in the change log
    #[inline(always)]
    fn fast_forward_proof(
        &self,
        leaf: &mut Node,
        proof: &mut [Node; MAX_DEPTH],
        leaf_index: u32,
        mut changelog_buffer_index: u64,
        use_full_buffer: bool,
    ) -> bool {
        solana_logging!(
            "Fast-forwarding proof, starting index {}",
            changelog_buffer_index
        );
        let mask: usize = MAX_BUFFER_SIZE - 1;

        let mut updated_leaf = *leaf;
        log_compute!();
        // Modifies proof by iterating through the change log
        loop {
            // If use_full_buffer is false, this loop will terminate if the initial value of changelog_buffer_index is the active index
            if !use_full_buffer && changelog_buffer_index == self.active_index {
                break;
            }
            changelog_buffer_index = (changelog_buffer_index + 1) & mask as u64;
            self.change_logs[changelog_buffer_index as usize].update_proof_or_leaf(
                leaf_index,
                proof,
                &mut updated_leaf,
            );
            // If use_full_buffer is true, this loop will do 1 full pass of the change logs
            if use_full_buffer && changelog_buffer_index == self.active_index {
                break;
            }
        }
        log_compute!();
        let proof_leaf_unchanged = updated_leaf == *leaf;
        *leaf = updated_leaf;
        proof_leaf_unchanged
    }

    #[inline(always)]
    fn find_root_in_changelog(&self, current_root: Node) -> Option<u64> {
        let mask: usize = MAX_BUFFER_SIZE - 1;
        for i in 0..self.buffer_size {
            let j = self.active_index.wrapping_sub(i) & mask as u64;
            if self.change_logs[j as usize].root == current_root {
                return Some(j);
            }
        }
        None
    }

    #[inline(always)]
    fn check_valid_leaf(
        &self,
        current_root: Node,
        leaf: Node,
        proof: &mut [Node; MAX_DEPTH],
        leaf_index: u32,
        allow_inferred_proof: bool,
    ) -> Result<bool, CMTError> {
        let mask: usize = MAX_BUFFER_SIZE - 1;
        let (changelog_index, use_full_buffer) = match self.find_root_in_changelog(current_root) {
            Some(matching_changelog_index) => (matching_changelog_index, false),
            None => {
                if allow_inferred_proof {
                    solana_logging!("Failed to find root in change log -> replaying full buffer");
                    (
                        self.active_index.wrapping_sub(self.buffer_size - 1) & mask as u64,
                        true,
                    )
                } else {
                    return Err(CMTError::RootNotFound);
                }
            }
        };
        let mut updatable_leaf_node = leaf;
        let proof_leaf_unchanged = self.fast_forward_proof(
            &mut updatable_leaf_node,
            proof,
            leaf_index,
            changelog_index,
            use_full_buffer,
        );
        if !proof_leaf_unchanged {
            return Err(CMTError::LeafContentsModified);
        }
        Ok(recompute(updatable_leaf_node, proof, leaf_index) == self.get_change_log().root)
    }

    /// Note: Enabling `allow_inferred_proof` will fast forward the given proof
    /// from the beginning of the buffer in the case that the supplied root is not in the buffer.
    #[inline(always)]
    fn try_apply_proof(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        proof: &mut [Node; MAX_DEPTH],
        leaf_index: u32,
        allow_inferred_proof: bool,
    ) -> Result<Node, CMTError> {
        solana_logging!("Active Index: {}", self.active_index);
        solana_logging!("Rightmost Index: {}", self.rightmost_proof.index);
        solana_logging!("Buffer Size: {}", self.buffer_size);
        solana_logging!("Leaf Index: {}", leaf_index);
        let valid_root =
            self.check_valid_leaf(current_root, leaf, proof, leaf_index, allow_inferred_proof)?;
        if !valid_root {
            return Err(CMTError::InvalidProof);
        }
        self.update_internal_counters();
        Ok(self.update_buffers_from_proof(new_leaf, proof, leaf_index))
    }

    /// Implements circular addition for changelog buffer index
    fn update_internal_counters(&mut self) {
        let mask: usize = MAX_BUFFER_SIZE - 1;
        self.active_index += 1;
        self.active_index &= mask as u64;
        if self.buffer_size < MAX_BUFFER_SIZE as u64 {
            self.buffer_size += 1;
        }
        self.sequence_number = self.sequence_number.saturating_add(1);
    }

    /// Creates a new root from a proof that is valid for the root at `self.active_index`
    fn update_buffers_from_proof(&mut self, start: Node, proof: &[Node], index: u32) -> Node {
        let change_log = &mut self.change_logs[self.active_index as usize];
        // Also updates change_log's current root
        let root = change_log.replace_and_recompute_path(index, start, proof);
        // Update rightmost path if possible
        if self.rightmost_proof.index < (1 << MAX_DEPTH) {
            if index < self.rightmost_proof.index as u32 {
                change_log.update_proof_or_leaf(
                    self.rightmost_proof.index - 1,
                    &mut self.rightmost_proof.proof,
                    &mut self.rightmost_proof.leaf,
                );
            } else {
                assert!(index == self.rightmost_proof.index);
                solana_logging!("Appending rightmost leaf");
                self.rightmost_proof.proof.copy_from_slice(&proof);
                self.rightmost_proof.index = index + 1;
                self.rightmost_proof.leaf = change_log.get_leaf();
            }
        }
        root
    }
}
