use anchor_lang::{
    emit,
    prelude::*,
    solana_program::{entrypoint::ProgramResult, log::sol_log_compute_units, keccak::hashv, sysvar::rent::Rent},
    Discriminator, Event,
};
use std::convert::AsRef;
use std::ops::Deref;
use std::ops::DerefMut;

declare_id!("DhpK18H3tzRBNWV6X4J4Cb9Z3Hm8MBxMAXVKJkc5aDj6");


/// Max number of concurrent changes to tree supported before having to regenerate proofs
#[constant]
pub const MAX_SIZE: usize = 512;

/// Max depth of the Merkle tree
#[constant]
pub const MAX_DEPTH: usize = 20;

#[constant]
pub const PADDING: usize = 32 - MAX_DEPTH;

/// Used for node parity when hashing
#[constant]
pub const MASK: usize = MAX_SIZE - 1;

pub const EMPTY: Node = Node {
    inner: [0 as u8; 32],
};

/// Calculates hash of empty nodes up to level i
pub fn empty_node(level: u32) -> Node {
    let mut data = EMPTY;
    if level != 0 {
        let lower_empty = empty_node(level - 1);
        let hash = hashv(&[lower_empty.as_ref(), lower_empty.as_ref()]);
        data.copy_from_slice(hash.as_ref());
    }
    data
}

/// Recomputes root of the Merkle tree from Node & proof
pub fn recompute(mut leaf: Node, proof: &[Node], index: u32) -> Node {
    msg!("Recompute");
    sol_log_compute_units();
    for (i, s) in proof.iter().enumerate() {
        if index >> i & 1 == 0 {
            let res = hashv(&[leaf.as_ref(), s.as_ref()]);
            leaf.copy_from_slice(res.as_ref());
        } else {
            let res = hashv(&[s.as_ref(), leaf.as_ref()]);
            leaf.copy_from_slice(res.as_ref());
        }
    }
    sol_log_compute_units();
    leaf
}

/// Inverts the path
pub fn index_to_path(index: u32) -> u32 {
    ((1 << MAX_DEPTH as u32) - 1) & (!index)
}

#[program]
pub mod gummyroll {
    use super::*;

    pub fn init_empty_gummyroll(ctx: Context<Initialize>) -> ProgramResult {
        let mut merkle_roll = ctx.accounts.merkle_roll.load_init()?;
        merkle_roll.initialize(ctx.accounts.authority.key())
    }

    pub fn init_gummyroll_with_root(
        ctx: Context<Initialize>,
        root: Node,
        leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> ProgramResult {
        let mut merkle_roll = ctx.accounts.merkle_roll.load_init()?;
        merkle_roll.initialize_with_root(ctx.accounts.authority.key(), root, leaf, proof, index)
    }

    pub fn replace_leaf(
        ctx: Context<Modify>,
        root: Node,
        previous_leaf: Node,
        new_leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> ProgramResult {
        let mut merkle_roll = ctx.accounts.merkle_roll.load_mut()?;
        match merkle_roll.set_leaf(root, previous_leaf, new_leaf, proof, index) {
            Some(new_root) => {
                msg!("New Root: {:?}", new_root);
                emit!(merkle_roll.get_change_log());
            }
            None => return Err(ProgramError::InvalidInstructionData),
        }
        Ok(())
    }

    pub fn append(ctx: Context<Modify>, leaf: Node) -> ProgramResult {
        let mut merkle_roll = ctx.accounts.merkle_roll.load_mut()?;
        match merkle_roll.append(leaf) {
            Some(new_root) => {
                msg!("New Root: {:?}", new_root);
                emit!(merkle_roll.get_change_log());
            }
            None => return Err(ProgramError::InvalidInstructionData),
        }
        Ok(())
    }

    pub fn insert_or_append(
        ctx: Context<Modify>,
        root: Node,
        leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> ProgramResult {
        let mut merkle_roll = ctx.accounts.merkle_roll.load_mut()?;
        match merkle_roll.fill_empty_or_append(root, leaf, proof, index) {
            Some(new_root) => {
                let change_log = merkle_roll.get_change_log();
                msg!("New Root: {:?}", new_root);
                msg!("Inserted Index - {:?}", change_log.index);
                emit!(change_log);
            }
            None => return Err(ProgramError::InvalidInstructionData),
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    pub merkle_roll: AccountLoader<'info, MerkleRoll>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    pub merkle_roll: AccountLoader<'info, MerkleRoll>,
    pub authority: Signer<'info>,
}

#[derive(Debug, Copy, Clone, AnchorDeserialize, AnchorSerialize, Default, PartialEq)]
pub struct Node {
    inner: [u8; 32],
}

impl Node {
    pub fn new(inner: [u8; 32]) -> Self {
        Self { inner }
    }
}

impl Deref for Node {
    type Target = [u8; 32];
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AsRef<[u8; 32]> for Node {
    fn as_ref(&self) -> &[u8; 32] {
        &self.inner
    }
}

impl From<[u8; 32]> for Node {
    fn from(inner: [u8; 32]) -> Self {
        Self { inner }
    }
}

#[derive(Default, Copy, Clone, PartialEq, AnchorDeserialize, AnchorSerialize)]
/// Stores proof for a given Merkle root update
pub struct ChangeLog {
    /// Nodes of off-chain merkle tree
    path: [Node; MAX_DEPTH],
    /// Bitmap of node parity (used when hashing)
    index: u32,
    _padding: u32,
}

impl Discriminator for ChangeLog {
    fn discriminator() -> [u8; 8] {
        [u8::MAX; 8]
    }
}

impl Event for ChangeLog {
    fn data(&self) -> Vec<u8> {
        self.try_to_vec().unwrap()
    }
}

impl ChangeLog {
    pub fn get_leaf(&self) -> Node {
        self.path[0]
    }

    pub fn recompute_path(&mut self, mut start: Node, proof: &[Node]) -> Node {
        self.path[0] = start;
        for (ix, s) in proof.iter().enumerate() {
            if self.index >> ix & 1 == 0 {
                let res = hashv(&[start.as_ref(), s.as_ref()]);
                start.copy_from_slice(res.as_ref());
            } else {
                let res = hashv(&[s.as_ref(), start.as_ref()]);
                start.copy_from_slice(res.as_ref());
            }
            if ix < MAX_DEPTH - 1 {
                self.path[ix + 1] = start;
            }
        }
        start
    }
}

#[derive(Default, Copy, Clone, PartialEq, AnchorDeserialize, AnchorSerialize)]
pub struct Path {
    proof: [Node; MAX_DEPTH],
    leaf: Node,
    index: u32,
    _padding: u32,
}

/// Tracks updates to off-chain Merkle tree
///
/// Allows for concurrent writes to same merkle tree so long as proof
/// was generated for a that has had at most MAX_SIZE updates since the tx was submitted
#[account(zero_copy)]
pub struct MerkleRoll {
    authority: Pubkey,
    /// Chronological roots of the off-chain Merkle tree stored in circular buffer
    roots: [Node; MAX_SIZE],
    /// Proof for respective root
    change_logs: [ChangeLog; MAX_SIZE],
    /// Index of most recent root & changes
    active_index: u64,
    /// Number of active changes we are tracking
    buffer_size: u64,
    rightmost_proof: Path,
}

impl MerkleRoll {
    pub fn initialize(&mut self, authority: Pubkey) -> ProgramResult {
        let mut rightmost_proof = Path::default();
        for (i, node) in rightmost_proof.proof.iter_mut().enumerate() {
            *node = empty_node(i as u32);
        }
        self.authority = authority;
        self.roots[0] = empty_node(MAX_DEPTH as u32);
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        Ok(())
    }

    pub fn initialize_with_root(
        &mut self,
        authority: Pubkey,
        root: Node,
        rightmost_leaf: Node,
        proof: [Node; MAX_DEPTH],
        index: u32,
    ) -> ProgramResult {
        let rightmost_proof = Path {
            proof,
            index: index + 1,
            leaf: rightmost_leaf,
            _padding: 0,
        };
        assert_eq!(root, recompute(rightmost_leaf, &proof, index));
        self.authority = authority;
        self.roots[0] = root;
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        Ok(())
    }

    /// Returns on-chain root
    pub fn get_root(&self) -> Node {
        self.roots[self.active_index as usize]
    }

    pub fn get_change_log(&self) -> ChangeLog {
        self.change_logs[self.active_index as usize]
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
            let hash = hashv(&[node.as_ref(), empty_node(i as u32).as_ref()]);
            node.copy_from_slice(hash.as_ref());
            let rightmost_hash = if ((self.rightmost_proof.index - 1) >> i) & 1 == 1 {
                hashv(&[
                    self.rightmost_proof.proof[i].as_ref(),
                    intersection_node.as_ref(),
                ])
            } else {
                hashv(&[
                    intersection_node.as_ref(),
                    self.rightmost_proof.proof[i].as_ref(),
                ])
            };
            intersection_node.copy_from_slice(rightmost_hash.as_ref());
            self.rightmost_proof.proof[i] = empty_node(i as u32);
        }

        // Compute the where the new node intersects the main tree
        change_list[intersection] = node;
        let hash = hashv(&[intersection_node.as_ref(), node.as_ref()]);
        node.copy_from_slice(hash.as_ref());
        self.rightmost_proof.proof[intersection] = intersection_node;

        // Update the change list path up to the root
        for i in intersection + 1..MAX_DEPTH {
            change_list[i] = node;
            let hash = if (self.rightmost_proof.index >> i) & 1 == 1 {
                hashv(&[self.rightmost_proof.proof[i].as_ref(), node.as_ref()])
            } else {
                hashv(&[node.as_ref(), self.rightmost_proof.proof[i].as_ref()])
            };
            node.copy_from_slice(hash.as_ref());
        }

        self.increment_active_index();
        self.roots[self.active_index as usize] = node;
        self.change_logs[self.active_index as usize] = ChangeLog {
            path: change_list,
            index: self.rightmost_proof.index,
            _padding: 0,
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
        sol_log_compute_units();
        let root = self.find_and_update_leaf(current_root, EMPTY, leaf, proof, index, true);
        sol_log_compute_units();
        root
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
        if index > self.rightmost_proof.index {
            msg!(
                "Received an index larger than the rightmost index {} > {}",
                index,
                self.rightmost_proof.index
            );
            None
        } else {
            sol_log_compute_units();
            let root = self.find_and_update_leaf(current_root, leaf, new_leaf, proof, index, false);
            sol_log_compute_units();
            root
        }
    }

    /// Internal function used to set leaf value & record changelog
    fn find_and_update_leaf(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        mut proof: [Node; MAX_DEPTH],
        index: u32,
        append_on_conflict: bool,
    ) -> Option<Node> {
        msg!("Active Index: {}", self.active_index);
        msg!("Rightmost Index: {}", self.rightmost_proof.index);
        msg!("Buffer Size: {}", self.buffer_size);
        msg!("Leaf Index: {}", index);
        for i in 0..self.buffer_size {
            let j = self.active_index.wrapping_sub(i) & MASK as u64;
            if self.roots[j as usize] != current_root {
                continue;
            }
            let old_root = recompute(leaf, &proof, index);
            if old_root == current_root && index > self.rightmost_proof.index && append_on_conflict
            {
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
                msg!("Invalid proof");
                return None;
            }
        }
        msg!("Failed to find root");
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
        mut j: u64,
        append_on_conflict: bool,
    ) -> Option<Node> {
        let mut updated_leaf = leaf;
        msg!("Fast-forwarding proof");
        sol_log_compute_units();
        while j != self.active_index {
            // Implement circular index addition
            j += 1;
            j &= MASK as u64;
            if index != self.change_logs[j as usize].index {
                let common_path_len = ((index ^ self.change_logs[j as usize].index) << PADDING)
                    .leading_zeros() as usize;
                let critbit_index = (MAX_DEPTH - 1) - common_path_len;
                proof[critbit_index] = self.change_logs[j as usize].path[critbit_index];
            } else {
                updated_leaf = self.change_logs[j as usize].get_leaf();
            }
        }
        sol_log_compute_units();
        if updated_leaf != leaf {
            if leaf == EMPTY && append_on_conflict {
                return self.append(new_leaf);
            } else {
                msg!("Leaf already updated");
                return None;
            }
        }
        self.increment_active_index();
        let new_root = self.apply_changes(new_leaf, proof, index);
        self.roots[self.active_index as usize] = new_root;
        Some(new_root)
    }

    fn increment_active_index(&mut self) {
        self.active_index += 1;
        self.active_index &= MASK as u64;
        if self.buffer_size < MAX_SIZE as u64 {
            self.buffer_size += 1;
        }
    }

    /// Creates a new root from a proof that is valid for the root at `self.active_index`
    fn apply_changes(&mut self, start: Node, proof: &[Node], index: u32) -> Node {
        let change_log = &mut self.change_logs[self.active_index as usize];
        change_log.index = index;
        let root = change_log.recompute_path(start, proof);
        if index < self.rightmost_proof.index as u32 {
            if index != self.rightmost_proof.index - 1 {
                let common_path_len = ((index ^ (self.rightmost_proof.index - 1) as u32) << PADDING)
                    .leading_zeros() as usize;
                let critbit_index = (MAX_DEPTH - 1) - common_path_len;
                self.rightmost_proof.proof[critbit_index] = change_log.path[critbit_index];
            }
        } else {
            assert!(index == self.rightmost_proof.index);
            msg!("Appending rightmost leaf");
            self.rightmost_proof.proof.copy_from_slice(&proof);
            self.rightmost_proof.index = index + 1;
            self.rightmost_proof.leaf = change_log.get_leaf();
        }
        root
    }
}
