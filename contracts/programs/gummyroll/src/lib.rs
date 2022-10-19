//! Gummyroll is an on-chain Merkle tree that supports concurrent writes.
//!
//! A buffer of proof-like changelogs is stored on-chain that allow multiple proof-based writes to succeed within the same slot.
//! This is accomplished by fast-forwarding out-of-date (or possibly invalid) proofs based on information stored in the changelogs.
//! See a copy of the whitepaper [here](https://google.com)
//!
//! While Gummyroll trees can generically store arbitrary information,
//! one exemplified use-case is the [Bubblegum](https://google.com) contract,
//! which uses Gummyroll trees to store encoded information about NFTs.
//! The use of Gummyroll within Bubblegum allows for:
//! - up to 1 billion NFTs to be stored in a single account on-chain (10,000x decrease in on-chain cost)
//! - (by default) up to 1024 concurrent updates per slot (this number is not correct)
//!
//! Operationally, Gummyroll trees **must** be supplemented by off-chain indexers to cache information
//! about leafs and to power an API that can supply up-to-date proofs to allow updates to the tree.
//! All modifications to Gummyroll trees are settled on the Solana ledger via instructions against the Gummyroll contract.
//! A production-ready indexer (Plerkle) can be found in the [Metaplex program library](https://google.com)

use anchor_lang::{
    emit,
    prelude::*,
    solana_program::sysvar::{clock::Clock, rent::Rent},
};
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::cast_slice_mut;
use concurrent_merkle_tree::{state::EMPTY, utils::empty_node_cached};
use std::mem::size_of;

pub mod error;
pub mod state;
pub mod utils;

use crate::error::GummyrollError;
use crate::state::{CandyWrapper, ChangeLogEvent, MerkleRollHeader};
use crate::utils::{wrap_event, ZeroCopy};
pub use concurrent_merkle_tree::{error::CMTError, merkle_roll::{MerkleRoll, MerkleInterface, MerkleRollPreAppend, PreAppendInterface}, state::Node};

declare_id!("GRoLLzvxpxxu2PGNJMMeZPyMxjAUH9pKqxGXV9DGiceU");

/// Context for initializing a new Merkle tere
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    /// CHECK: This account will be zeroed out, and the size will be validated
    pub merkle_roll: UncheckedAccount<'info>,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,

    /// Program used to emit changelogs as instruction data.
    /// See `WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh`
    pub candy_wrapper: Program<'info, CandyWrapper>,
}
#[derive(Accounts)]
/// Context to modify the Pre-Append data structure including init, reset or push partition
pub struct ModifyPreAppend<'info> {
    /// CHECK: validated in instruction
    pub merkle_roll: UncheckedAccount<'info>,

    #[account(mut, seeds = [merkle_roll.key().as_ref()], bump)]
    /// CHECK: validated in instruction, besides seeds.
    pub subtree_append: UncheckedAccount<'info>,

    /// Authority over merkle_roll
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,

    /// Program used to emit changelogs as instruction data.
    /// See `WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh`
    pub candy_wrapper: Program<'info, CandyWrapper>,
}

#[derive(Accounts)]
/// Context to append a subtree to this Gummyroll tree
pub struct AppendSubtree<'info> {
    #[account(mut)]
    /// CHECK: validated in instruction
    pub merkle_roll: UncheckedAccount<'info>,

    /// CHECK: validated in instruction
    pub subtree_merkle_roll: UncheckedAccount<'info>,

    /// CHECK: validated in instruction
    #[account(seeds = [subtree_merkle_roll.key().as_ref()], bump)]
    pub subtree_append: UncheckedAccount<'info>,

    /// Authority over merkle_roll
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,

    /// Program used to emit changelogs as instruction data.
    /// See `WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh`
    pub candy_wrapper: Program<'info, CandyWrapper>,
}

/// Context for inserting, appending, or replacing a leaf in the tree
#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_roll: UncheckedAccount<'info>,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,

    /// Program used to emit changelogs as instruction data.
    /// See `WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh`
    pub candy_wrapper: Program<'info, CandyWrapper>,
}

/// Context for validating a provided proof against the Merkle tree.
/// Throws an error if provided proof is invalid.
#[derive(Accounts)]
pub struct VerifyLeaf<'info> {
    /// CHECK: This account is validated in the instruction
    pub merkle_roll: UncheckedAccount<'info>,
}

/// Context for transferring `authority`
#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(mut)]
    /// CHECK: This account is validated in the instruction
    pub merkle_roll: UncheckedAccount<'info>,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Signer<'info>,
}

#[inline(always)]
fn check_canopy_bytes(canopy_bytes: &mut [u8]) -> Result<()> {
    if canopy_bytes.len() % size_of::<Node>() != 0 {
        msg!(
            "Canopy byte length {} is not a multiple of {}",
            canopy_bytes.len(),
            size_of::<Node>()
        );
        err!(GummyrollError::CanopyLengthMismatch)
    } else {
        Ok(())
    }
}

#[inline(always)]
fn get_cached_path_length(canopy: &mut [Node], max_depth: u32) -> Result<u32> {
    // The offset of 2 is applied because the canopy is a full binary tree without the root node
    // Size: (2^n - 2) -> Size + 2 must be a power of 2
    let closest_power_of_2 = (canopy.len() + 2) as u32;
    // This expression will return true if `closest_power_of_2` is actually a power of 2
    if closest_power_of_2 & (closest_power_of_2 - 1) == 0 {
        // (1 << max_depth) returns the number of leaves in the full merkle tree
        // (1 << (max_depth + 1)) - 1 returns the number of nodes in the full tree
        // The canopy size cannot exceed the size of the tree
        if closest_power_of_2 > (1 << (max_depth + 1)) {
            msg!(
                "Canopy size is too large. Size: {}. Max size: {}",
                closest_power_of_2 - 2,
                (1 << (max_depth + 1)) - 2
            );
            return err!(GummyrollError::CanopyLengthMismatch);
        }
    } else {
        msg!(
            "Canopy length {} is not 2 less than a power of 2",
            canopy.len()
        );
        return err!(GummyrollError::CanopyLengthMismatch);
    }
    // 1 is subtracted from the trailing zeros because the root is not stored in the canopy
    Ok(closest_power_of_2.trailing_zeros() - 1)
}

fn update_canopy(
    canopy_bytes: &mut [u8],
    max_depth: u32,
    change_log: Option<Box<ChangeLogEvent>>,
) -> Result<()> {
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice_mut::<u8, Node>(canopy_bytes);
    let path_len = get_cached_path_length(canopy, max_depth)?;
    if let Some(cl) = change_log {
        // Update the canopy from the newest change log
        for path_node in cl.path.iter().rev().skip(1).take(path_len as usize) {
            // node_idx - 2 maps to the canopy index
            canopy[(path_node.index - 2) as usize] = path_node.node;
        }
    }
    Ok(())
}

fn fill_in_proof_from_canopy(
    canopy_bytes: &mut [u8],
    max_depth: u32,
    index: u32,
    proof: &mut Vec<Node>,
) -> Result<()> {
    // 26 is hard coded as it is the current max depth that Gummyroll supports
    let mut empty_node_cache = Box::new([EMPTY; 30]);
    check_canopy_bytes(canopy_bytes)?;
    let canopy = cast_slice_mut::<u8, Node>(canopy_bytes);
    let path_len = get_cached_path_length(canopy, max_depth)?;
    // We want to compute the node index (w.r.t. the canopy) where the current path
    // intersects the leaves of the canopy
    let mut node_idx = ((1 << max_depth) + index) >> (max_depth - path_len);
    let mut inferred_nodes = vec![];
    while node_idx > 1 {
        // node_idx - 2 maps to the canopy index
        let shifted_index = node_idx as usize - 2;
        let cached_idx = if shifted_index % 2 == 0 {
            shifted_index + 1
        } else {
            shifted_index - 1
        };
        if canopy[cached_idx] == EMPTY {
            let level = max_depth - (31 - node_idx.leading_zeros());
            let empty_node = empty_node_cached::<30>(level, &mut empty_node_cache);
            canopy[cached_idx] = empty_node;
            inferred_nodes.push(empty_node);
        } else {
            inferred_nodes.push(canopy[cached_idx]);
        }
        node_idx >>= 1;
    }
    // We only want to add inferred canopy nodes such that the proof length
    // is equal to the tree depth. If the lengh of proof + lengh of canopy nodes is
    // less than the tree depth, the instruction will fail.
    let overlap = (proof.len() + inferred_nodes.len()).saturating_sub(max_depth as usize);
    proof.extend(inferred_nodes.iter().skip(overlap));
    Ok(())
}

/// This applies a given function on a merkle roll by
/// allowing the compiler to infer the size of the tree based
/// upon the header information stored on-chain
macro_rules! merkle_roll_get_size {
    ($header:ident) => {
        // Note: max_buffer_size MUST be a power of 2
        match ($header.max_depth, $header.max_buffer_size) {
            (3, 8) => Ok(size_of::<MerkleRoll<3, 8>>()),
            (5, 8) => Ok(size_of::<MerkleRoll<5, 8>>()),
            (14, 64) => Ok(size_of::<MerkleRoll<14, 64>>()),
            (14, 256) => Ok(size_of::<MerkleRoll<14, 256>>()),
            (14, 1024) => Ok(size_of::<MerkleRoll<14, 1024>>()),
            (14, 2048) => Ok(size_of::<MerkleRoll<14, 2048>>()),
            (20, 64) => Ok(size_of::<MerkleRoll<20, 64>>()),
            (20, 256) => Ok(size_of::<MerkleRoll<20, 256>>()),
            (20, 1024) => Ok(size_of::<MerkleRoll<20, 1024>>()),
            (20, 2048) => Ok(size_of::<MerkleRoll<20, 2048>>()),
            (24, 64) => Ok(size_of::<MerkleRoll<24, 64>>()),
            (24, 256) => Ok(size_of::<MerkleRoll<24, 256>>()),
            (24, 512) => Ok(size_of::<MerkleRoll<24, 512>>()),
            (24, 1024) => Ok(size_of::<MerkleRoll<24, 1024>>()),
            (24, 2048) => Ok(size_of::<MerkleRoll<24, 2048>>()),
            (26, 512) => Ok(size_of::<MerkleRoll<26, 512>>()),
            (26, 1024) => Ok(size_of::<MerkleRoll<26, 1024>>()),
            (26, 2048) => Ok(size_of::<MerkleRoll<26, 2048>>()),
            (30, 512) => Ok(size_of::<MerkleRoll<30, 512>>()),
            (30, 1024) => Ok(size_of::<MerkleRoll<30, 1024>>()),
            (30, 2048) => Ok(size_of::<MerkleRoll<30, 2048>>()),
            _ => {
                msg!(
                    "Failed to get size of max depth {} and max buffer size {}",
                    $header.max_depth,
                    $header.max_buffer_size
                );
                err!(GummyrollError::MerkleRollConstantsError)
            }
        }
    };
}

/// Returns the size of a merkle_roll's associated pre-append data structure
macro_rules! merkle_roll_append_get_size {
    ($header:ident) => {
        match ($header.max_depth) {
            3 => Ok(size_of::<MerkleRollPreAppend<4>>()),
            5 => Ok(size_of::<MerkleRollPreAppend<6>>()),
            14 => Ok(size_of::<MerkleRollPreAppend<15>>()),
            20 => Ok(size_of::<MerkleRollPreAppend<21>>()),
            24 => Ok(size_of::<MerkleRollPreAppend<25>>()),
            26 => Ok(size_of::<MerkleRollPreAppend<27>>()),
            30 => Ok(size_of::<MerkleRollPreAppend<31>>()),
            _ => {
                msg!(
                    "Failed to get size of pre append struct for max_depth {}",
                    $header.max_depth
                );
                err!(GummyrollError::MerkleRollConstantsError)
            }
        }
    };
}

macro_rules! merkle_roll_interface_for_size {
    ($max_depth: literal, $max_buffer_size: literal, $bytes: ident) => {
        match MerkleRoll::<$max_depth,$max_buffer_size>::load_mut_bytes($bytes) {
            Ok(merkle_roll) => { Ok(merkle_roll as &mut dyn MerkleInterface) }
            Err(err) => {
                msg!("Error zero copying merkle roll: {}", err);
                err!(GummyrollError::ZeroCopyError)
            }
        }
    }
}

fn get_merkle_roll_interface(max_depth: u32, max_buffer_size: u32, bytes: &mut [u8]) -> Result<&mut dyn MerkleInterface> {
    match (max_depth, max_buffer_size) {
        (3, 8) => merkle_roll_interface_for_size!(3, 8, bytes),
        (5, 8) => merkle_roll_interface_for_size!(5, 8, bytes),
        (14, 64) => merkle_roll_interface_for_size!(14, 64, bytes),
        (14, 256) => merkle_roll_interface_for_size!(14, 256, bytes),
        (14, 1024) => merkle_roll_interface_for_size!(14, 1024, bytes),
        (14, 2048) => merkle_roll_interface_for_size!(14, 2048, bytes),
        (20, 64) => merkle_roll_interface_for_size!(20, 64, bytes),
        (20, 256) => merkle_roll_interface_for_size!(20, 256, bytes),
        (20, 1024) => merkle_roll_interface_for_size!(20, 1024, bytes),
        (20, 2048) => merkle_roll_interface_for_size!(20, 2048, bytes),
        (24, 64) => merkle_roll_interface_for_size!(24, 64, bytes),
        (24, 256) => merkle_roll_interface_for_size!(24, 256, bytes),
        (24, 512) => merkle_roll_interface_for_size!(24, 512, bytes),
        (24, 1024) => merkle_roll_interface_for_size!(24, 1024, bytes),
        (24, 2048) => merkle_roll_interface_for_size!(24, 2048, bytes),
        (26, 512) => merkle_roll_interface_for_size!(26, 512, bytes),
        (26, 1024) => merkle_roll_interface_for_size!(26, 1024, bytes),
        (26, 2048) => merkle_roll_interface_for_size!(26, 2048, bytes),
        (30, 512) => merkle_roll_interface_for_size!(30, 512, bytes),
        (30, 1024) => merkle_roll_interface_for_size!(30, 1024, bytes),
        (30, 2048) => merkle_roll_interface_for_size!(30, 2048, bytes),
        _ => {
            msg!(
                "max depth {} and max buffer size {} are an unsupported combination",
                max_depth,
                max_buffer_size
            );
            err!(GummyrollError::MerkleRollConstantsError)
        }
    }
}

macro_rules! pre_append_interface_for_size {
    ($num_partitions: literal, $bytes: ident) => {
        match MerkleRollPreAppend::<$num_partitions>::load_mut_bytes($bytes) {
            Ok(merkle_roll_pre_append) => { Ok(merkle_roll_pre_append as &mut dyn PreAppendInterface) }
            Err(err) => {
                msg!("Error zero copying pre append data structure: {}", err);
                err!(GummyrollError::PreAppendZeroCopyError)
            }
        }
    }
}

fn get_pre_append_interface(max_depth: u32, bytes: &mut [u8]) -> Result<&mut dyn PreAppendInterface> {
    match max_depth {
        3 => pre_append_interface_for_size!(4, bytes),
        5 => pre_append_interface_for_size!(6, bytes),
        14 => pre_append_interface_for_size!(15, bytes),
        20 => pre_append_interface_for_size!(21, bytes),
        24 => pre_append_interface_for_size!(24, bytes),
        26 => pre_append_interface_for_size!(26, bytes),
        30 => pre_append_interface_for_size!(31, bytes),
        _ => {
            msg!(
                "Failed to get size of append for max_depth {}",
                max_depth
            );
            err!(GummyrollError::MerkleRollConstantsError)
        }
    }
}

fn get_changelog_after_op(merkle_roll_obj: &dyn MerkleInterface, id: Pubkey) -> Box::<ChangeLogEvent> {
    Box::<ChangeLogEvent>::from((merkle_roll_obj.get_change_log(), id, merkle_roll_obj.get_sequence_number()))
}

#[program]
pub mod gummyroll {
    use super::*;

    /// Creates a new merkle tree with maximum leaf capacity of `power(2, max_depth)`
    /// and a minimum concurrency limit of `max_buffer_size`.
    ///
    /// Concurrency limit represents the # of replace instructions that can be successfully
    /// executed with proofs dated for the same root. For example, a maximum buffer size of 1024
    /// means that a minimum of 1024 replaces can be executed before a new proof must be
    /// generated for the next replace instruction.
    ///
    /// Concurrency limit should be determined by empirically testing the demand for
    /// state built on top of gummyroll.
    pub fn init_empty_gummyroll(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        header.initialize(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            Clock::get()?.slot,
        );
        header.serialize(&mut header_bytes)?;
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);
        let id = ctx.accounts.merkle_roll.key();
        let mut merkle_roll_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;
        merkle_roll_obj.initialize();
        let change_log = get_changelog_after_op(merkle_roll_obj, id);
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.candy_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, None);
        Ok(())
    }

    /// Note:
    /// Supporting this instruction open a security vulnerability for indexers.
    /// This instruction has been deemed unusable for publicly indexed compressed NFTs.
    /// Indexing batched data in this way requires indexers to read in the `uri`s onto physical storage
    /// and then into their database. This opens up a DOS attack vector, whereby this instruction is
    /// repeatedly invoked, causing indexers to fail.
    pub fn init_gummyroll_with_root(
        ctx: Context<Initialize>,
        max_depth: u32,
        max_buffer_size: u32,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
        _changelog_db_uri: String,
        _metadata_db_uri: String,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        header.initialize(
            max_depth,
            max_buffer_size,
            &ctx.accounts.authority.key(),
            Clock::get()?.slot,
        );
        header.serialize(&mut header_bytes)?;
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);

        // Get rightmost proof from accounts
        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        assert_eq!(proof.len(), max_depth as usize);

        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::initialize_with_root(root, leaf, proof, index)
        let mut merkle_roll_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;
        merkle_roll_obj.initialize_with_root(root, leaf, &proof, index);
        let change_log = get_changelog_after_op(merkle_roll_obj, id);
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.candy_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }
    
    // TODO(sorend): We need to handle errors thrown by the PreAppend and MerkleRoll methods
    /// Initializes the subtree append data structure for a given Gummyroll tree
    pub fn init_or_reset_subtree_append_account(
        ctx: Context<ModifyPreAppend>
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, _) = rest.split_at_mut(merkle_roll_size);
        let merkle_roll_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;

        // Assert that subtree_append is the correct size for header.max_depth
        let merkle_roll_append_size = merkle_roll_append_get_size!(header)?;
        let mut subtree_append_account_bytes = ctx.accounts.subtree_append.try_borrow_mut_data()?;
        let (_, mut pre_append_struct_data) = subtree_append_account_bytes.split_at_mut(8);
        assert!(pre_append_struct_data.len() == merkle_roll_append_size, "Append account has incorrect size");
        let mut pre_append_obj = get_pre_append_interface(header.max_depth, pre_append_struct_data)?;
        pre_append_obj.reset(merkle_roll_obj);
        Ok(())
    }

    /// Push a partition to the pre-append data structure
    pub fn push_pre_append_partition(
        ctx: Context<ModifyPreAppend>,
        rightmost_leaf: Node,
        rightmost_proof: Vec<Node>
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;

        let (mut header_bytes, rest) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, _) = rest.split_at_mut(merkle_roll_size);
        let merkle_roll_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;

        // The current authority for the merkle_roll must sign be a signer
        assert_eq!(header.authority, ctx.accounts.authority.key());

        // Assert that subtree_append is the correct size for header.max_depth
        let merkle_roll_append_size = merkle_roll_append_get_size!(header)?;
        let mut subtree_append_account_bytes = ctx.accounts.subtree_append.try_borrow_mut_data()?;
        let (_, mut pre_append_struct_data) = subtree_append_account_bytes.split_at_mut(8);
        assert!(pre_append_struct_data.len() == merkle_roll_append_size, "Append account has incorrect size");
        let mut pre_append_obj = get_pre_append_interface(header.max_depth, pre_append_struct_data)?;
        pre_append_obj.push_partition(merkle_roll_obj, rightmost_leaf, &rightmost_proof);
        Ok(())
    }

    /// Append a subtree to a larger merkle roll in a byte dense way. Requires that the subtree's pre append data structure is initialized.
    pub fn append_subtree(
        ctx: Context<AppendSubtree>
    ) -> Result<()> {
        // 1. load mutable bytes for merkle_roll to append to
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (mut header_bytes, rest) =
            merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());
        let mut header = Box::new(MerkleRollHeader::try_from_slice(&header_bytes)?);
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, _) = rest.split_at_mut(merkle_roll_size);
        let mut merkle_roll_receiver_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;

        // 2. load bytes for subtree_merkle_roll
        let mut subtree_merkle_roll_bytes = ctx.accounts.subtree_merkle_roll.try_borrow_mut_data()?;
        let (mut subtree_header_bytes, subtree_rest) =
            subtree_merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());
        let mut subtree_header = Box::new(MerkleRollHeader::try_from_slice(&subtree_header_bytes)?);
        let subtree_merkle_roll_size = merkle_roll_get_size!(subtree_header)?;
        let (subtree_roll_bytes, _) = subtree_rest.split_at_mut(subtree_merkle_roll_size);
        let mut merkle_roll_to_append = get_merkle_roll_interface(subtree_header.max_depth, subtree_header.max_buffer_size, subtree_roll_bytes)?;
        
        // 3. load bytes for subtree_append struct
        let merkle_roll_append_size = merkle_roll_append_get_size!(subtree_header)?;
        let mut pre_append_struct_bytes = ctx.accounts.subtree_append.try_borrow_mut_data()?;
        let (_, mut pre_append_struct_data) = pre_append_struct_bytes.split_at_mut(8);
        assert!(pre_append_struct_data.len() == merkle_roll_append_size, "Append account has incorrect size");
        let pre_append_obj = get_pre_append_interface(subtree_header.max_depth, pre_append_struct_data)?;

        assert!(merkle_roll_to_append.get_sequence_number() == pre_append_obj.get_sequence_number(), "tree to append changed since partitions pushed, invalid to append");

        merkle_roll_receiver_obj.append_subtree_packed(
            &pre_append_obj.get_rightmost_proofs_as_vec(), 
            &pre_append_obj.get_rightmost_leaves_as_vec(), 
            &pre_append_obj.get_rightmost_leaves_as_vec()
        );

        Ok(())
    }

    /// Executes an instruction that overwrites a leaf node.
    /// Composing programs should check that the data hashed into previous_leaf
    /// matches the authority information necessary to execute this instruction.
    pub fn replace_leaf(
        ctx: Context<Modify>,
        root: [u8; 32],
        previous_leaf: [u8; 32],
        new_leaf: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, rest) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        let id = ctx.accounts.merkle_roll.key();
        // A call is made to MerkleRoll::set_leaf(root, previous_leaf, new_leaf, proof, index)
        let mut merkle_roll_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;
        merkle_roll_obj.set_leaf(root, previous_leaf, new_leaf, &proof, index);
        let change_log = get_changelog_after_op(merkle_roll_obj, id);
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.candy_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }

    /// Transfers `authority`
    /// Requires `authority` to sign
    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (mut header_bytes, _) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let mut header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());

        header.authority = new_authority;
        msg!("Authority transferred to: {:?}", header.authority);
        header.serialize(&mut header_bytes)?;

        Ok(())
    }

    /// Verifies a provided proof and leaf.
    /// If invalid, throws an error.
    pub fn verify_leaf(
        ctx: Context<VerifyLeaf>,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, rest) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());
        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        let id = ctx.accounts.merkle_roll.key();

        let mut merkle_roll_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;
        merkle_roll_obj.prove_leaf(root, leaf, &proof, index);
        Ok(())
    }

    /// This instruction allows the tree's `authority` to append a new leaf to the tree
    /// without having to supply a valid proof.
    ///
    /// This is accomplished by using the rightmost_proof of the merkle roll to construct a
    /// valid proof, and then updating the rightmost_proof for the next leaf if possible.
    pub fn append(ctx: Context<Modify>, leaf: [u8; 32]) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, rest) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());

        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());

        let id = ctx.accounts.merkle_roll.key();
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);
        let mut merkle_roll_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;
        merkle_roll_obj.append(leaf);
        let change_log = get_changelog_after_op(merkle_roll_obj, id);
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.candy_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }

    /// This instruction takes a proof, and will attempt to write the given leaf
    /// to the specified index in the tree. If the insert operation fails, the leaf will be `append`-ed
    /// to the tree.
    /// It is up to the indexer to parse the final location of the leaf from the emitted changelog.
    pub fn insert_or_append(
        ctx: Context<Modify>,
        root: [u8; 32],
        leaf: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let mut merkle_roll_bytes = ctx.accounts.merkle_roll.try_borrow_mut_data()?;
        let (header_bytes, rest) = merkle_roll_bytes.split_at_mut(size_of::<MerkleRollHeader>());
        let header = Box::new(MerkleRollHeader::try_from_slice(header_bytes)?);
        assert_eq!(header.authority, ctx.accounts.authority.key());
        let merkle_roll_size = merkle_roll_get_size!(header)?;
        let (roll_bytes, canopy_bytes) = rest.split_at_mut(merkle_roll_size);

        let mut proof = vec![];
        for node in ctx.remaining_accounts.iter() {
            proof.push(node.key().to_bytes());
        }
        fill_in_proof_from_canopy(canopy_bytes, header.max_depth, index, &mut proof)?;
        // A call is made to MerkleRoll::fill_empty_or_append
        let id = ctx.accounts.merkle_roll.key();
        let mut merkle_roll_obj = get_merkle_roll_interface(header.max_depth, header.max_buffer_size, roll_bytes)?;
        merkle_roll_obj.fill_empty_or_append(root, leaf, &proof, index);
        let change_log = get_changelog_after_op(merkle_roll_obj, id);
        wrap_event(change_log.try_to_vec()?, &ctx.accounts.candy_wrapper)?;
        emit!(*change_log);
        update_canopy(canopy_bytes, header.max_depth, Some(change_log))
    }
}
