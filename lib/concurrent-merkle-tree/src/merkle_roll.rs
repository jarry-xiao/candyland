use crate::{
    error::CMTError,
    state::{ChangeLog, ChangeLogInterface, Node, Path, EMPTY},
    utils::{empty_node, empty_node_cached, fill_in_proof, hash_to_parent, recompute},
};
use bytemuck::{Pod, Zeroable};
use borsh::{BorshDeserialize, BorshSerialize};
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

pub trait MerkleInterface {
    fn get_sequence_number(&self) -> u64;
    fn get_rightmost_proof_node_at_index(&self, i: usize) -> Result<[u8; 32], CMTError>;
    fn set_rightmost_proof_node_at_index(&mut self, i: usize, node_to_write: [u8; 32]) -> Result<(), CMTError>;
    fn get_rightmost_proof_index(&self) -> u32;
    fn get_rightmost_proof_leaf(&self) -> [u8; 32];
    fn get_max_depth(&self) -> usize;
    fn initialize(&mut self) -> Result<Node, CMTError>;

    fn initialize_with_root(
        &mut self,
        root: Node,
        rightmost_leaf: Node,
        proof_vec: &Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError>;
    fn get_change_log(&self) -> Box<dyn ChangeLogInterface>;

    fn prove_leaf(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof_vec: &Vec<Node>,
        leaf_index: u32,
    ) -> Result<Node, CMTError>;

    fn append(&mut self, node: Node) -> Result<Node, CMTError>;

    fn append_subtree_direct(
        &mut self,
        subtree_root: Node,
        subtree_rightmost_leaf: Node,
        subtree_rightmost_index: u32,
        subtree_rightmost_proof: &Vec<Node>,
    ) -> Result<Node, CMTError>;

    fn append_subtree_packed(
        &mut self,
        subtree_proofs: &Vec<Vec<Node>>,
        subtree_rightmost_leaves: &Vec<Node>,
        subtree_roots: &Vec<Node>
    ) -> Result<Node, CMTError>;

    fn fill_empty_or_append(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof_vec: &Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError>;

    fn set_leaf(
        &mut self,
        current_root: Node,
        previous_leaf: Node,
        new_leaf: Node,
        proof_vec: &Vec<Node>,
        index: u32,
    ) -> Result<Node, CMTError>;
}

/// Tracks updates to off-chain Merkle tree
///
/// Allows for concurrent writes to same merkle tree so long as proof
/// was generated at most MAX_SIZE updates since the tx was submitted
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize)]
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

impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> MerkleInterface for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE> {
    fn get_sequence_number(&self) -> u64 {
        return self.sequence_number;
    }

    fn get_rightmost_proof_node_at_index(&self, i: usize) -> Result<[u8; 32], CMTError> {
        if i < self.rightmost_proof.proof.len() {
            return Ok(self.rightmost_proof.proof[i]);
        } else {
            return Err(CMTError::InvalidProofAccessOrWrite);
        }
    }

    fn set_rightmost_proof_node_at_index(&mut self, i: usize, node_to_write: [u8; 32]) -> Result<(), CMTError> {
        if i < self.rightmost_proof.proof.len() {
            self.rightmost_proof.proof[i] = node_to_write;
            return Ok(());
        } else {
            return Err(CMTError::InvalidProofAccessOrWrite);
        }
    }

    fn get_rightmost_proof_leaf(&self) -> [u8; 32] {
        return self.rightmost_proof.leaf;
    }

    fn get_rightmost_proof_index(&self) -> u32 {
        return self.rightmost_proof.index;
    }

    fn get_max_depth(&self) -> usize {
        return MAX_DEPTH;
    }

    fn initialize(&mut self) -> Result<Node, CMTError> {
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

    fn initialize_with_root(
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

    fn get_change_log(&self) -> Box<dyn ChangeLogInterface> {
        Box::new(self.change_logs[self.active_index as usize])
    }

    fn prove_leaf(
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

    fn append(&mut self, node: Node) -> Result<Node, CMTError> {
        let mut mut_node = node;
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        if mut_node == EMPTY {
            return Err(CMTError::CannotAppendEmptyNode);
        }
        if self.rightmost_proof.index >= 1 << MAX_DEPTH {
            return Err(CMTError::TreeFull);
        }
        if self.rightmost_proof.index == 0 {
            return self.initialize_tree_from_append(mut_node, self.rightmost_proof.proof);
        }
        let leaf = mut_node.clone();
        let intersection = self.rightmost_proof.index.trailing_zeros() as usize;
        let mut change_list = [EMPTY; MAX_DEPTH];
        let mut intersection_node = self.rightmost_proof.leaf;
        let mut empty_node_cache = Box::new([Node::default(); MAX_DEPTH]);

        for i in 0..MAX_DEPTH {
            change_list[i] = mut_node;
            if i < intersection {
                // Compute proof to the appended node from empty nodes
                let sibling = empty_node_cached::<MAX_DEPTH>(i as u32, &mut empty_node_cache);
                hash_to_parent(
                    &mut intersection_node,
                    &self.rightmost_proof.proof[i],
                    ((self.rightmost_proof.index - 1) >> i) & 1 == 0,
                );
                hash_to_parent(&mut mut_node, &sibling, true);
                self.rightmost_proof.proof[i] = sibling;
            } else if i == intersection {
                // Compute the where the new node intersects the main tree
                hash_to_parent(&mut mut_node, &intersection_node, false);
                self.rightmost_proof.proof[intersection] = intersection_node;
            } else {
                // Update the change list path up to the root
                hash_to_parent(
                    &mut mut_node,
                    &self.rightmost_proof.proof[i],
                    ((self.rightmost_proof.index - 1) >> i) & 1 == 0,
                );
            }
        }
        self.update_state_from_append(mut_node, change_list, self.rightmost_proof.index, leaf)?;
        Ok(mut_node)
    }

    /// Append subtree to current tree
    fn append_subtree_direct(
        &mut self,
        subtree_root: Node,
        subtree_rightmost_leaf: Node,
        subtree_rightmost_index: u32,
        subtree_rightmost_proof: &Vec<Node>,
    ) -> Result<Node, CMTError> {
        check_bounds(MAX_DEPTH, MAX_BUFFER_SIZE);
        if self.rightmost_proof.index >= 1 << MAX_DEPTH {
            return Err(CMTError::TreeFull);
        }

        // If the rightmost proof is empty, then we just append the rightmost leaf
        if subtree_rightmost_proof.len() == 0 {
            return self.append(subtree_rightmost_leaf);
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
            return self.initialize_tree_from_subtree_append(
                subtree_root,
                subtree_rightmost_leaf,
                subtree_rightmost_index,
                subtree_rightmost_proof,
            );
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
        self.update_state_from_append(
            node,
            change_list,
            self.rightmost_proof.index + subtree_rightmost_index - 1,
            leaf,
        )?;
        Ok(node)
    }

    /// @dev: this function appends a subtree of any size at any time (as long as there is sufficient capacity in the large tree, but irrespective of the next free index)
    ///       this requires the subtree (with 2^n leaves) to be unpacked into n+1 sub-subtrees of size (# leaves), 1,1,..,2^(n-2),2^(n-1)
    ///       as the consumer of this function you must pass the root, right most leaf and right most proof to the sub-subtree of size 2^(n-1) [as the last element in each of the vector parameters], etc. where the first element is a tree of size 1
    /// @notice: in all cases, this function is much slower and more expensive than its counterpart "append_subtree_direct". The tradeoff is that this
    ///          method will always succeed as long as the larger tree has enough total capacity to fit the subtree being appended, where as a direct append will fail
    ///          if the subtree cannot be appended by appending its complete topological form starting with the next available index in the larger tree
    fn append_subtree_packed(
        &mut self,
        subtree_proofs: &Vec<Vec<Node>>,
        subtree_rightmost_leaves: &Vec<Node>,
        subtree_roots: &Vec<Node>
    ) -> Result<Node, CMTError> {
        // 1. determine index range for each sub-subtree to append of size 2^k
        //    a. find the index at which to insert 2^(n-1): m
        //    b. compute m-l (leftmost index). Insert trees based on the binary representation of m-l (starting with the lsbs) -> each 1 in the binary representation of m-l maps to a sub-subtree to be appended
        let n = subtree_proofs.len()-1;
        let largest_power: u32 = 1 << (n-1);

        // TODO(sorend): to be more efficient we can use a u32 as a bitset rather than a vector of bool == vector of char (8x inefficient)
        let mut index_appended = vec![false; n+1];
        // find the first index in constant time => some i st i % (2^(n-1)) == 0
        let mut largest_tree_insertion_index = self.rightmost_proof.index;
        let remainder = largest_tree_insertion_index % largest_power;
        if remainder != 0 {
            largest_tree_insertion_index = largest_tree_insertion_index + (largest_power - remainder);
        }

        let first_difference = largest_tree_insertion_index - self.rightmost_proof.index;
        let num_bits = first_difference.count_ones() + first_difference.count_zeros();

        // Append trees ascending up to largest_tree_insertion_index
        for i in 0..num_bits {
            // If the i'th lsb is a 1, then we want to insert the appropriate tree
            if first_difference >> i & 1 == 1 {
                let ind = (i+1) as usize;
                self.append_subtree_direct(subtree_roots[ind], subtree_rightmost_leaves[ind], 1 << subtree_proofs[ind].len(), &subtree_proofs[ind])?;
                index_appended[ind] = true;
            }
        }

        // 2. append the largest sub-subtree at index m
        // Append the largest sub-subtree at the largest tree insertion index
        self.append_subtree_direct(subtree_roots[n], subtree_rightmost_leaves[n], 1 << subtree_proofs[n].len(), &subtree_proofs[n])?;
        index_appended[n] = true;

        // 3. append the rest of the remaining sub-subtrees in order from largest to smallest which have not yet been inserted
        // Iterate over the remaining sub-subtrees that need to be appended in order from largest to smallest (indices [n-1, 0])
        let mut root = EMPTY;
        for i in (0..n).rev() {
            if !index_appended[i] {
                root = self.append_subtree_direct(subtree_roots[i], subtree_rightmost_leaves[i], 1 << subtree_proofs[i].len(), &subtree_proofs[i])?;
            }
        }

        Ok(root)
    }

    /// Convenience function for `set_leaf`
    /// On write conflict:
    /// Will append
    fn fill_empty_or_append(
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
    fn set_leaf(
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
        rmp_leaf: Node,
    ) -> Result<(), CMTError> {
        self.update_internal_counters();
        self.change_logs[self.active_index as usize] =
            ChangeLog::<MAX_DEPTH>::new(root, change_list, rmp_index);
        self.rightmost_proof.index = rmp_index + 1;
        self.rightmost_proof.leaf = rmp_leaf;
        Ok(())
    }

    fn initialize_tree_from_subtree_append(
        &mut self,
        subtree_root: Node,
        subtree_rightmost_leaf: Node,
        subtree_rightmost_index: u32,
        subtree_rightmost_proof: &Vec<Node>,
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
        self.update_state_from_append(
            node,
            change_list,
            self.rightmost_proof.index + subtree_rightmost_index - 1,
            leaf,
        )?;
        Ok(node)
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
        Ok(recompute(updatable_leaf_node, proof, leaf_index) == self.get_change_log().get_root())
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

#[inline(always)]
fn assert_valid_pre_append_state(merkle_roll: &dyn MerkleInterface, num_partitions: usize) {
    assert!(num_partitions == merkle_roll.get_max_depth()+1, "Must partition tree into MAX_DEPTH+1 partitions");
    assert!(merkle_roll.get_rightmost_proof_index() >= 1 << merkle_roll.get_max_depth(), "Cannot begin constructing pre-append data structure before tree is full");
}

pub trait PreAppendInterface {
    fn get_sequence_number(&self) -> u64;
    fn get_rightmost_proofs_as_vec(&self) -> Vec<Vec<Node>>;
    fn get_rightmost_leaves_as_vec(&self) -> Vec<Node>;
    fn get_partition_roots_as_vec(&self) -> Vec<Node>;
    fn get_initialized(&self) -> bool;
    fn reset(&mut self, merkle_roll: &dyn MerkleInterface) -> Result<(), CMTError>;
    fn push_partition(&mut self, merkle_roll: &dyn MerkleInterface, rightmost_leaf: Node, rightmost_proof: &Vec<Node>) -> Result<(), CMTError>;
}
/// Stores merkle tree partitions in storage due to transaction size constraints.
///
/// If a user wants to append their tree as a subtree to some other tree. They should populate this structure.
/// This structure should be filled by providing partitions with the following number of leaves in order: 1,1,2,4,...2^(k+1) where 2^k is the number of leaves in the associated merkle_roll struct
/// The subtrees should be passed in order from smallest to largest, greedily taking the smaller trees from the rightmost side of the tree, such that the root of each partitioned tree aligns with the ith index into the rightmost proof of the larger tree
#[derive(Copy, Clone, BorshDeserialize, BorshSerialize)]
pub struct MerkleRollPreAppend<const NUM_PARTITIONS: usize> {
    /// The next depth of tree which needs to be initialized  
    pub next_index_to_initialize: u32,
    /// Whether or not the structure has been fully initialized with NUM_PARTITIONS trees
    pub initialized: u32,
    /// The sequence number of the MerkleRoll when the initialization of this struct began
    pub sequence_number: u64,
    /// Right most proofs to each partitioned subtree
    pub rightmost_proofs: [Path<NUM_PARTITIONS>; NUM_PARTITIONS],
    /// Rightmost leaves of each partitioned subtree
    pub partition_rightmost_leaves: [Node; NUM_PARTITIONS],
    /// Roots of partitioned trees
    pub partition_roots: [Node; NUM_PARTITIONS]
}

unsafe impl<const NUM_PARTITIONS: usize> Zeroable
    for MerkleRollPreAppend<NUM_PARTITIONS>
{
}
unsafe impl<const NUM_PARTITIONS: usize> Pod
    for MerkleRollPreAppend<NUM_PARTITIONS>
{
}

impl<const NUM_PARTITIONS: usize> PreAppendInterface for MerkleRollPreAppend<NUM_PARTITIONS> {
    fn get_sequence_number(&self) -> u64 {
        return self.sequence_number;
    }

    fn get_rightmost_proofs_as_vec(&self) -> Vec<Vec<Node>> {
        let proofs_vec: Vec<Vec<Node>> = vec![vec![], vec![]];
        for i in 2..NUM_PARTITIONS {
            let mut proof: Vec<Node> = vec![];
            for j in 0..i-1 {
                proof.push(self.rightmost_proofs[i].proof[j]);
            }
        }
        return proofs_vec;
    }

    fn get_rightmost_leaves_as_vec(&self) -> Vec<Node> {
        let mut leaf_vec: Vec<Node> = vec![];
        for leaf in self.partition_rightmost_leaves {
            leaf_vec.push(leaf);
        }
        return leaf_vec;
    }

    fn get_partition_roots_as_vec(&self) -> Vec<Node> {
        let mut root_vec: Vec<Node> = vec![];
        for root in self.partition_roots {
            root_vec.push(root);
        }
        return root_vec;
    }

    fn get_initialized(&self) -> bool {
        return self.initialized == 1;
    }

    /// Reset the pre-append data structure to a zero-state configuration.
    /// @dev: doubles as an initialization function
    fn reset(&mut self, merkle_roll: &dyn MerkleInterface) -> Result<(), CMTError> {
        assert_valid_pre_append_state(merkle_roll, NUM_PARTITIONS);
        self.set_empty_values(merkle_roll)?;
        Ok(())
    }

    /// @dev: Push a new tree partition. We expect partitions to be pushed with num leaves 1,1,2,4.. IN THAT ORDER. The first partitioned tree should be the rightmost_leaf of merkle_roll associated with this struct.
    ///       The second partition pushed should be the second rightmost leaf in the tree. Followed by the rightmost subtree of size 2 which has not yet been included etc.
    ///       While you are pushing partitions to this struct, the associated merkle_roll should not change. If it does it would invalidate some of the pushed state, and you should call reset and begin pushing partitions from the beginning. 
    fn push_partition(&mut self, merkle_roll: &dyn MerkleInterface, rightmost_leaf: Node, rightmost_proof: &Vec<Node>) -> Result<(), CMTError> {
        assert_valid_pre_append_state(merkle_roll, NUM_PARTITIONS);
        assert!(merkle_roll.get_sequence_number() == self.sequence_number, "Tree has been modified while pushing proofs. State invalid, please reset.");
        let index = self.next_index_to_initialize as usize;
        if index == 0 {
            assert!(rightmost_proof.len() == 0);
            assert!(merkle_roll.get_rightmost_proof_leaf() == rightmost_leaf);
        } 
        else if index == 1 {
            assert!(rightmost_proof.len() == 0);
            assert!(merkle_roll.get_rightmost_proof_node_at_index(0)? == rightmost_leaf);
        }
        else {
            assert!(rightmost_proof.len() == index - 1);
            assert!(recompute(rightmost_leaf, &rightmost_proof[..], (1 << index) - 1) == merkle_roll.get_rightmost_proof_node_at_index(index-1)?);
            // For the single leaves, we don't need to update their roots since they are just leaves by themselves
            self.partition_roots[index] = merkle_roll.get_rightmost_proof_node_at_index(index-1)?;
        }
        // Copy the supplied proof into the data structure. Note that indices from rightmost_proof.len() -> NUM_PARTITIONS will be garbage and should be ignored in whatever is passed to append_direct
        for i in 0..rightmost_proof.len() {
            self.rightmost_proofs[index].proof[i] = rightmost_proof[i];
        }
        self.partition_rightmost_leaves[index] = rightmost_leaf;
        self.next_index_to_initialize += 1;
        if self.next_index_to_initialize as usize >= NUM_PARTITIONS {
            self.initialized = 1;
        }
        Ok(())
    }
}

impl<const NUM_PARTITIONS: usize> MerkleRollPreAppend<NUM_PARTITIONS> {
    // TODO(sorend): investigate if there is a way to map all MerkleRoll types that match with constant MAX_DEPTH to the same function, rather than creating a new function for each buffer size which doesn't impact the pre-append data structure
    pub fn new(merkle_roll: &dyn MerkleInterface) -> Self {
        Self {
            rightmost_proofs: [Path::<NUM_PARTITIONS>::default(); NUM_PARTITIONS],
            partition_rightmost_leaves: [Node::default(); NUM_PARTITIONS],
            partition_roots: [Node::default(); NUM_PARTITIONS],
            next_index_to_initialize: 0,
            initialized: 0,
            sequence_number: merkle_roll.get_sequence_number()
        }
    }

    fn set_empty_values(&mut self, merkle_roll: &dyn MerkleInterface) -> Result<(), CMTError> {
        assert_valid_pre_append_state(merkle_roll, NUM_PARTITIONS);
        self.rightmost_proofs = [Path::<NUM_PARTITIONS>::default(); NUM_PARTITIONS];
        self.partition_rightmost_leaves = [Node::default(); NUM_PARTITIONS];
        self.partition_roots = [Node::default(); NUM_PARTITIONS];
        self.next_index_to_initialize = 0;
        self.initialized = 0;
        self.sequence_number = merkle_roll.get_sequence_number();
        Ok(())
    }
}
