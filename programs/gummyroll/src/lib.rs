use anchor_lang::{
    prelude::*,
    solana_program::{
        keccak::hashv,
        program::{invoke_signed},
        system_instruction,
        sysvar::rent::Rent
    }
};
use std::convert::AsRef;
use std::ops::Deref;
use std::ops::DerefMut;
use std::mem::size_of;

// declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
declare_id!("21qUUADZpCsa37m4fXSybmVJWxtX3DZAkh422RRZ9e9t");

// type Node = [u8; 32];

/// Max number of concurrent changes to tree supported before having to regenerate proofs
// pub const MAX_SIZE: u64 = 64;

/// Max depth of the Merkle tree
// pub const MAX_DEPTH: u64 = 20;

// pub const PADDING: u64 = 32 - MAX_DEPTH;

/// Used for node parity when hashing
// pub const MASK: u64 = 64 - 1;

/// Calculates hash of empty nodes up to level i
pub fn empty_node(level: u32) -> Node {
    let mut data = Node::new([0; 32]);
    if level != 0 {
        let lower_empty = empty_node(level - 1);
        let hash = hashv(&[lower_empty.as_ref(), lower_empty.as_ref()]);
        data.copy_from_slice(hash.as_ref());
    }
    data
}

/// Recomputes root of the Merkle tree from Node & proof
pub fn recompute(mut leaf: Node, proof: &[Node], path: u32) -> Node {
    for (ix, s) in proof.iter().enumerate() {
        if path >> ix & 1 == 1 {
            let res = hashv(&[leaf.as_ref(), s.as_ref()]);
            leaf.copy_from_slice(res.as_ref());
        } else {
            let res = hashv(&[s.as_ref(), leaf.as_ref()]);
            leaf.copy_from_slice(res.as_ref());
        }
    }
    leaf
}

/// Inverts the path
pub fn indexToPath(index: u32) -> u32 {
    ((1 << 20) - 1) & (!index)
}

#[program]
pub mod gummyroll {
    use super::*;

    pub fn init_gummyroll(ctx: Context<Initialize>, root: Node) -> Result<()> {
        msg!(&format!("node: {:?}", root.inner));
        let mut merkle_roll = ctx.accounts.merkle_roll.load_init()?;
        merkle_roll.initialize(root, ctx.accounts.payer.key());
        msg!(&format!("merkle roll root: {:?}", *merkle_roll.roots[0]));
        Ok(())
    }

    pub fn replace_leaf(ctx: Context<ReplaceLeaf>, root: Node, previous_leaf: Node, new_leaf: Node, proof: [Node; 20], index: u32) -> Result<()> {
        let mut merkle_roll = ctx.accounts.merkle_roll.load_mut()?;
        let path = indexToPath(index);
        msg!(&format!("Root: {:?}", root.inner));
        msg!(&format!("Index: {:?}", index));
        msg!(&format!("Path: {:?} (leading zeros: {})", path, path.leading_zeros()));

        // Copy argument data to make mutable copy (needed for efficient fast-forwarding)
        let mut mutable_proof = [Node::default(); 20];
        mutable_proof.copy_from_slice(&proof);

        merkle_roll.replace(root, previous_leaf, new_leaf, proof, path);
        msg!(&format!("New root: {:?}", merkle_roll.get().inner)); 
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    pub merkle_roll: AccountLoader<'info, MerkleRoll>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReplaceLeaf<'info> {
    #[account(mut)]
    pub merkle_roll: AccountLoader<'info, MerkleRoll>,
    #[account(mut)]
    pub payer: Signer<'info>,
}

#[derive(Copy, Clone, Default, AnchorSerialize, AnchorDeserialize)]
/// Stores proof for a given Merkle root update
pub struct ChangeLog {
    /// Nodes of off-chain merkle tree
    changes: [Node; 20],
    /// Bitmap of node parity (used when hashing)
    path: u32,
    _padding: u32,
}

/// Tracks updates to off-chain Merkle tree
///
/// Allows for concurrent writes to same merkle tree so long as proof
/// was generated for a that has had at most 64 updates since the tx was submitted
#[account(zero_copy)]
pub struct MerkleRoll {
    authority: Pubkey,
    /// Chronological roots of the off-chain Merkle tree stored in circular buffer
    roots: [Node; 64],
    /// Proof for respective root
    change_logs: [ChangeLog; 64],
    /// Index of most recent root & changes
    active_index: u64,
    /// Number of active changes we are tracking
    buffer_size: u64,
}

impl Default for MerkleRoll {
    fn default() -> Self {
        Self {
            authority: Pubkey::new(&[0; 32]),
            roots: [Node::new([0; 32]); 64],
            change_logs: [ChangeLog {
                changes: [Node::new([0; 32]); 20],
                path: 0,
                _padding: 0
            }; 64],
            active_index: 0,
            buffer_size: 1,
        }
    }
}

impl MerkleRoll {
    pub fn initialize(&mut self, root: Node, authority: Pubkey) {
        self.roots = [empty_node(20_u32); 64];
        self.roots[0] = root;
        self.authority = authority;
        self.change_logs = [ChangeLog {
            changes: [Node::new([0; 32]); 20],
            path: 0,
            _padding: 0,
        }; 64];
        self.active_index = 0;
        self.buffer_size = 1;
    }

    /// Returns on-chain root
    pub fn get(&self) -> Node {
        self.roots[self.active_index as usize]
    }

    pub fn add(
        &mut self,
        current_root: Node,
        leaf: Node,
        mut proof: [Node; 20],
        path: u32,
    ) -> Option<Node> {
        if self.buffer_size == 0 {
            let old_root = recompute(Node::new([0; 32]), &proof, path);
            if old_root == empty_node(20_u32) {
                return Some(self.update_and_apply_proof(leaf, &mut proof, path, 0));
            } else {
                println!("Bad proof");
                return None;
            }
        }
        self.replace(current_root, Node::new([0; 32]), leaf, proof, path)
    }

    pub fn remove(
        &mut self,
        current_root: Node,
        leaf: Node,
        proof: [Node; 20],
        path: u32,
    ) -> Option<Node> {
        self.replace(current_root, leaf, Node::new([0; 32]), proof, path)
    }

    pub fn replace(
        &mut self,
        current_root: Node,
        leaf: Node,
        new_leaf: Node,
        mut proof: [Node; 20],
        path: u32,
    ) -> Option<Node> {
        for i in 0..self.buffer_size {
            let j = self.active_index.wrapping_sub(i) & 63;

            if self.roots[j as usize] != current_root {
                if self.change_logs[j as usize].changes[20 - 1] == leaf {
                    return None;
                }
                continue;
            }
            let old_root = recompute(leaf, &proof, path);
            if old_root == current_root {
                return Some(self.update_and_apply_proof(new_leaf, &mut proof, path, j));
            } else {
                msg!(&format!("Recomputed old root: {:?}", old_root.inner));
                msg!(&format!("Expected root      : {:?}", current_root.inner));
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
        proof: &mut [Node; 20],
        path: u32,
        mut j: u64,
    ) -> Node {
        while j != self.active_index {
            // Implement circular index addition
            j += 1;
            j &= 63;

            // Calculate the index to the first differing node between current proof & changelog
            let path_len =
                ((path ^ self.change_logs[j as usize].path) << 12).leading_zeros() as usize;

            // Skip updates to current proof if we encounter a proof for the same index
            if path_len == 32 {
                continue;
            }

            // Calculate index to the node in current proof that needs updating from change log
            let critbit_index = (20 - 1) - path_len;
            proof[critbit_index] = self.change_logs[j as usize].changes[critbit_index];
        }

        // Only time we don't update active_index is for the first modification made
        // to a tree created from new_with_root
        if self.buffer_size > 0 {
            self.active_index += 1;
            self.active_index &= 63;
        }

        if self.buffer_size < 64 {
            self.buffer_size += 1;
        }

        let new_root = self.apply_changes(leaf, proof, path, self.active_index);
        self.roots[self.active_index as usize] = new_root;
        new_root
    }

    /// Creates a new root from a proof that is valid for the root at `self.active_index`
    /// Saves hashed nodes for new root in change log
    fn apply_changes(&mut self, mut start: Node, proof: &[Node], path: u32, i: u64) -> Node {
        let change_log = &mut self.change_logs[i as usize];
        change_log.changes[0] = start;
        for (ix, s) in proof.iter().enumerate() {
            if path >> ix & 1 == 1 {
                let res = hashv(&[start.as_ref(), s.as_ref()]);
                start.copy_from_slice(res.as_ref());
            } else {
                let res = hashv(&[s.as_ref(), start.as_ref()]);
                start.copy_from_slice(res.as_ref());
            }
            if ix < 20 - 1 {
                change_log.changes[ix + 1] = start;
            }
        }
        change_log.path = path;
        start
    }
}

#[derive(Copy, Clone, AnchorDeserialize, AnchorSerialize, Default, PartialEq)]
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
