use anchor_lang::{
    emit,
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult, keccak::hashv, log::sol_log_compute_units,
        program_error::ProgramError, sysvar::rent::Rent,
    },
};
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, PodCastError, Zeroable};
use std::any::type_name;
use std::convert::AsRef;
use std::mem::size_of;
use std::ops::Deref;
use std::ops::DerefMut;

declare_id!("GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD");

macro_rules! merkle_roll_depth_size_apply_fn {
    ($max_depth:literal, $max_size:literal, $bytes:ident, $func:ident, $($arg:tt)*) => {
        if size_of::<MerkleRoll::<$max_depth, $max_size>>() != $bytes.len() {
            msg!("Received account of invalid length");
            None
        } else {
            match MerkleRoll::<$max_depth, $max_size>::load_mut_bytes($bytes) {
                Ok(merkle_roll) => merkle_roll.$func($($arg)*),
                Err(e) => {
                    msg!("Error zero copying merkle roll {}", e);
                    None
                }
            }
        }
    }
}

macro_rules! merkle_roll_apply_fn {
    ($header:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        match ($header.max_depth, $header.max_buffer_size) {
            (14, 64) => merkle_roll_depth_size_apply_fn!(14, 64, $bytes, $func, $($arg)*),
            (14, 256) => merkle_roll_depth_size_apply_fn!(14, 256, $bytes, $func, $($arg)*),
            (14, 1024) => merkle_roll_depth_size_apply_fn!(14, 1024, $bytes, $func, $($arg)*),
            (14, 2448) => merkle_roll_depth_size_apply_fn!(14, 2448, $bytes, $func, $($arg)*),
            (16, 64) => merkle_roll_depth_size_apply_fn!(16, 64, $bytes, $func, $($arg)*),
            (16, 256) => merkle_roll_depth_size_apply_fn!(16, 256, $bytes, $func, $($arg)*),
            (16, 1024) => merkle_roll_depth_size_apply_fn!(16, 1024, $bytes, $func, $($arg)*),
            (16, 2448) => merkle_roll_depth_size_apply_fn!(16, 2448, $bytes, $func, $($arg)*),
            (18, 64) => merkle_roll_depth_size_apply_fn!(18, 64, $bytes, $func, $($arg)*),
            (18, 256) => merkle_roll_depth_size_apply_fn!(18, 256, $bytes, $func, $($arg)*),
            (18, 1024) => merkle_roll_depth_size_apply_fn!(18, 1024, $bytes, $func, $($arg)*),
            (18, 2448) => merkle_roll_depth_size_apply_fn!(18, 2448, $bytes, $func, $($arg)*),
            (20, 64) => merkle_roll_depth_size_apply_fn!(20, 64, $bytes, $func, $($arg)*),
            (20, 256) => merkle_roll_depth_size_apply_fn!(20, 256, $bytes, $func, $($arg)*),
            (20, 1024) => merkle_roll_depth_size_apply_fn!(20, 1024, $bytes, $func, $($arg)*),
            (20, 2448) => merkle_roll_depth_size_apply_fn!(20, 2448, $bytes, $func, $($arg)*),
            (22, 64) => merkle_roll_depth_size_apply_fn!(22, 64, $bytes, $func, $($arg)*),
            (22, 256) => merkle_roll_depth_size_apply_fn!(22, 256, $bytes, $func, $($arg)*),
            (22, 1024) => merkle_roll_depth_size_apply_fn!(22, 1024, $bytes, $func, $($arg)*),
            (22, 2448) => merkle_roll_depth_size_apply_fn!(22, 2448, $bytes, $func, $($arg)*),
            _ => {
                msg!("Failed to apply {} on merkle roll with max depth {} and max buffer size {}", stringify!($func), $header.max_depth, $header.max_buffer_size);
                None
            }
        }
    };
}

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

fn set_header<'a>(
    mut header_bytes: &'a mut [u8],
    max_depth: u32,
    max_buffer_size: u32,
    authority: &Pubkey,
) -> Result<MerkleRollHeader> {
    let mut header = MerkleRollHeader::try_from_slice(header_bytes)?;
    // Check header is empty
    assert_eq!(header.max_buffer_size, 0);
    assert_eq!(header.max_depth, 0);

    header.max_buffer_size = max_buffer_size;
    header.max_depth = max_depth;
    header.authority = *authority;
    header.serialize(&mut header_bytes)?;
    Ok(header)
}

fn load_header<'a>(header_bytes: &'a [u8]) -> Result<MerkleRollHeader> {
    let header = MerkleRollHeader::try_from_slice(header_bytes)?;
    Ok(header)
}

fn load_and_check_header(header_bytes: &[u8], authority: Pubkey) -> Result<MerkleRollHeader> {
    let header = load_header(header_bytes)?;
    assert_eq!(header.authority, authority);
    Ok(header)
}

#[program]
pub mod gummyroll {
    use super::*;

    pub fn init_empty_gummyroll(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> ProgramResult {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = set_header(
            header_bytes,
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
        )?;
        match merkle_roll_apply_fn!(header, roll_bytes, initialize,) {
            Some(new_root) => {
                msg!("New Root: {:?}", new_root);
                Ok(())
            }
            None => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn init_gummyroll_with_root(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
        root: Node,
        leaf: Node,
        index: u32,
    ) -> ProgramResult {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = set_header(
            header_bytes,
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
        )?;

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(Node {
                inner: node.key().to_bytes(),
            });
        }
        assert_eq!(proof.len(), max_depth as usize);

        match merkle_roll_apply_fn!(
            header,
            roll_bytes,
            initialize_with_root,
            root,
            leaf,
            proof,
            index
        ) {
            Some(new_root) => {
                msg!("New Root: {:?}", new_root);
                Ok(())
            }
            None => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn replace_leaf(
        ctx: Context<Modify>,
        root: Node,
        previous_leaf: Node,
        new_leaf: Node,
        index: u32,
    ) -> ProgramResult {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = load_and_check_header(header_bytes, ctx.accounts.authority.key())?;

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(Node {
                inner: node.key().to_bytes(),
            });
        }
        assert_eq!(proof.len(), header.max_depth as usize);

        match merkle_roll_apply_fn!(
            header,
            roll_bytes,
            set_leaf,
            root,
            previous_leaf,
            new_leaf,
            proof,
            index
        ) {
            Some(new_root) => {
                msg!("New Root: {:?}", new_root);
                Ok(())
            }
            None => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn append(ctx: Context<Modify>, leaf: Node) -> ProgramResult {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = load_and_check_header(header_bytes, ctx.accounts.authority.key())?;

        match merkle_roll_apply_fn!(header, roll_bytes, append, leaf) {
            Some(new_root) => {
                msg!("New Root: {:?}", new_root);
                Ok(())
            }
            None => Err(ProgramError::InvalidInstructionData),
        }
    }

    pub fn insert_or_append(
        ctx: Context<Modify>,
        root: Node,
        leaf: Node,
        index: u32,
    ) -> ProgramResult {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, roll_bytes) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = load_and_check_header(header_bytes, ctx.accounts.authority.key())?;

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(Node {
                inner: node.key().to_bytes(),
            });
        }
        assert_eq!(proof.len(), header.max_depth as usize);

        match merkle_roll_apply_fn!(
            header,
            roll_bytes,
            fill_empty_or_append,
            root,
            leaf,
            proof,
            index
        ) {
            Some(new_root) => {
                msg!("New Root: {:?}", new_root);
            }
            None => return Err(ProgramError::InvalidInstructionData),
        }
        Ok(())
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct MerkleRollHeader {
    pub max_buffer_size: u32,
    pub max_depth: u32,
    pub authority: Pubkey,
    // pub byte_vec: Vec<u8>
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    /// CHECK: unsafe :P
    pub merkle_roll: UncheckedAccount<'info>,
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

#[event]
pub struct ChangeLogEvent {
    /// Nodes of off-chain merkle tree
    path: Vec<Node>,
    /// Bitmap of node parity (used when hashing)
    index: u32,
}

#[derive(Copy, Clone, PartialEq)]
/// Stores proof for a given Merkle root update
#[repr(C)]
pub struct ChangeLog<const MAX_DEPTH: usize> {
    /// Historical root value before Path was applied
    root: Node,
    /// Nodes of off-chain merkle tree
    path: [Node; MAX_DEPTH],
    /// Bitmap of node parity (used when hashing)
    index: u32,
    _padding: u32,
}

impl<const MAX_DEPTH: usize> ChangeLog<MAX_DEPTH> {
    pub fn to_event(&self) -> ChangeLogEvent {
        ChangeLogEvent {
            path: self.path.to_vec(),
            index: self.index,
        }
    }

    pub fn get_leaf(&self) -> Node {
        self.path[0]
    }

    /// Sets all change log values from a leaf and valid proof
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
        self.root = start;
        start
    }
}

#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Path<const MAX_DEPTH: usize> {
    proof: [Node; MAX_DEPTH],
    leaf: Node,
    index: u32,
    _padding: u32,
}

impl<const MAX_DEPTH: usize> Default for Path<MAX_DEPTH> {
    fn default() -> Self {
        Self {
            proof: [Node::default(); MAX_DEPTH],
            leaf: Node::default(),
            index: 0,
            _padding: 0,
        }
    }
}

/// Tracks updates to off-chain Merkle tree
///
/// Allows for concurrent writes to same merkle tree so long as proof
/// was generated for a that has had at most MAX_SIZE updates since the tx was submitted
#[derive(Copy, Clone)]
pub struct MerkleRoll<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> {
    /// Index of most recent root & changes
    active_index: u64,
    /// Number of active changes we are tracking
    buffer_size: u64,
    /// Proof for respective root
    change_logs: [ChangeLog<MAX_DEPTH>; MAX_BUFFER_SIZE],
    rightmost_proof: Path<MAX_DEPTH>,
}

fn error_msg<T>(data_len: usize) -> impl Fn(PodCastError) -> ProgramError {
    move |_: PodCastError| -> ProgramError {
        msg!(
            "Failed to load {}. Size is {}, expected {}",
            type_name::<T>(),
            data_len,
            size_of::<T>(),
        );
        ProgramError::InvalidAccountData
    }
}

unsafe impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Zeroable
    for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}
unsafe impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> Pod
    for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}
impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> ZeroCopy
    for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}

pub trait ZeroCopy: Pod {
    fn load_mut_bytes<'a>(data: &'a mut [u8]) -> Result<&'a mut Self> {
        let size = size_of::<Self>();
        let data_len = data.len();

        Ok(bytemuck::try_from_bytes_mut(&mut data[..size])
            .map_err(error_msg::<Self>(data_len))
            .unwrap())
    }
}

impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE> {
    pub fn initialize(&mut self) -> Option<Node> {
        let mut rightmost_proof = Path::default();
        for (i, node) in rightmost_proof.proof.iter_mut().enumerate() {
            *node = empty_node(i as u32);
        }
        self.change_logs[0].root = empty_node(MAX_DEPTH as u32);
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        Some(self.change_logs[0].root)
    }

    pub fn initialize_with_root(
        &mut self,
        root: Node,
        rightmost_leaf: Node,
        proof_vec: Vec<Node>,
        index: u32,
    ) -> Option<Node> {
        let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
        proof.copy_from_slice(&proof_vec[..]);
        let rightmost_proof = Path {
            proof,
            index: index + 1,
            leaf: rightmost_leaf,
            _padding: 0,
        };
        assert_eq!(root, recompute(rightmost_leaf, &proof, index));
        self.change_logs[0].root = root;
        self.active_index = 0;
        self.buffer_size = 1;
        self.rightmost_proof = rightmost_proof;
        Some(root)
    }

    pub fn get_change_log(&self) -> ChangeLog<MAX_DEPTH> {
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
        self.change_logs[self.active_index as usize] = ChangeLog::<MAX_DEPTH> {
            root: node,
            path: change_list,
            index: self.rightmost_proof.index,
            _padding: 0,
        };
        emit!(self.change_logs[self.active_index as usize].to_event());
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
        proof_vec: Vec<Node>,
        index: u32,
    ) -> Option<Node> {
        let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
        proof.copy_from_slice(&proof_vec[..]);
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
        previous_leaf: Node,
        new_leaf: Node,
        proof_vec: Vec<Node>,
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
            let mut proof: [Node; MAX_DEPTH] = [Node::default(); MAX_DEPTH];
            proof.copy_from_slice(&proof_vec[..]);
            sol_log_compute_units();
            let root = self.find_and_update_leaf(
                current_root,
                previous_leaf,
                new_leaf,
                proof,
                index,
                false,
            );
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
        let mask: usize = MAX_BUFFER_SIZE - 1;

        for i in 0..self.buffer_size {
            let j = self.active_index.wrapping_sub(i) & mask as u64;
            if self.change_logs[j as usize].root != current_root {
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
        let mask: usize = MAX_BUFFER_SIZE - 1;
        let padding: usize = 32 - MAX_DEPTH;
        sol_log_compute_units();
        while j != self.active_index {
            // Implement circular index addition
            j += 1;
            j &= mask as u64;
            if index != self.change_logs[j as usize].index {
                let common_path_len = ((index ^ self.change_logs[j as usize].index) << padding)
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
        Some(self.apply_changes(new_leaf, proof, index))
    }

    fn increment_active_index(&mut self) {
        let mask: usize = MAX_BUFFER_SIZE - 1;

        self.active_index += 1;
        self.active_index &= mask as u64;
        if self.buffer_size < MAX_BUFFER_SIZE as u64 {
            self.buffer_size += 1;
        }
    }

    /// Creates a new root from a proof that is valid for the root at `self.active_index`
    fn apply_changes(&mut self, start: Node, proof: &[Node], index: u32) -> Node {
        let padding: usize = 32 - MAX_DEPTH;
        let change_log = &mut self.change_logs[self.active_index as usize];
        change_log.index = index;

        // Also updates change_log's current root
        let root = change_log.recompute_path(start, proof);

        emit!(change_log.to_event());

        // Update rightmost path if possible
        if self.rightmost_proof.index < (1 << MAX_DEPTH) {
            if index < self.rightmost_proof.index as u32 {
                if index != self.rightmost_proof.index - 1 {
                    let common_path_len = ((index ^ (self.rightmost_proof.index - 1) as u32) << padding)
                        .leading_zeros() as usize;
                    msg!("Common path len {}", common_path_len);
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
        }
        root
    }
}
