//! State related to storing a buffer of Merkle tree roots on-chain.
//!
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use concurrent_merkle_tree::state::{ChangeLog, ChangeLogInterface, Node, Path};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct PathNode {
    pub node: [u8; 32],
    pub index: u32,
}

impl PathNode {
    pub fn new(tree_node: Node, index: u32) -> Self {
        Self {
            node: tree_node,
            index,
        }
    }
}

#[event]
pub struct NewLeafEvent {
    /// Public key of the merkle roll
    pub id: Pubkey,
    pub leaf: [u8; 32],
}

#[event]
pub struct ChangeLogEvent {
    /// Public key of the Merkle Roll
    pub id: Pubkey,

    /// Nodes of off-chain merkle tree
    pub path: Vec<PathNode>,

    /// Index corresponding to the number of successful operations on this tree.
    /// Used by the off-chain indexer to figure out when there are gaps to be backfilled.
    pub seq: u64,

    /// Bitmap of node parity (used when hashing)
    pub index: u32,
}

impl From<(Box<dyn ChangeLogInterface>, Pubkey, u64)>
    for Box<ChangeLogEvent>
{
    fn from(log_info: (Box<dyn ChangeLogInterface>, Pubkey, u64)) -> Self {
        let (changelog, tree_id, seq) = log_info;
        let path_len = changelog.get_path_as_vec().len() as u32;
        let mut path: Vec<PathNode> = changelog
            .get_path_as_vec()
            .iter()
            .enumerate()
            .map(|(lvl, n)| {
                PathNode::new(
                    *n,
                    (1 << (path_len - lvl as u32)) + (changelog.get_index() >> lvl),
                )
            })
            .collect();
        path.push(PathNode::new(changelog.get_root(), 1));
        Box::new(ChangeLogEvent {
            id: tree_id,
            path,
            seq,
            index: changelog.get_index(),
        })
    }
}

/// Initialization parameters for a Gummyroll Merkle tree.
///
/// Only the following permutations are valid:
///
/// | max_depth | max_buffer_size       |
/// | --------- | --------------------- |
/// | 14        | (64, 256, 1024, 2048) |           
/// | 20        | (64, 256, 1024, 2048) |           
/// | 24        | (64, 256, 512, 1024, 2048) |           
/// | 26        | (64, 256, 512, 1024, 2048) |           
/// | 30        | (512, 1024, 2048) |           
///
#[derive(BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct MerkleRollHeader {
    /// Buffer of changelogs stored on-chain.
    /// Must be a power of 2; see above table for valid combinations.
    pub max_buffer_size: u32,

    /// Depth of the Merkle tree to store.
    /// Tree capacity can be calculated as power(2, max_depth).
    /// See above table for valid options.
    pub max_depth: u32,

    /// Authority that validates the content of the trees.
    /// Typically a program, e.g., the Bubblegum contract validates that leaves are valid NFTs.
    pub authority: Pubkey,

    /// Slot corresponding to when the Merkle tree was created.
    /// Provides a lower-bound on what slot to start (re-)building a tree from.
    pub creation_slot: u64,
}

impl MerkleRollHeader {
    pub fn initialize(
        &mut self,
        max_depth: u32,
        max_buffer_size: u32,
        authority: &Pubkey,
        creation_slot: u64,
    ) {
        // Check header is empty
        assert_eq!(self.max_buffer_size, 0);
        assert_eq!(self.max_depth, 0);
        self.max_buffer_size = max_buffer_size;
        self.max_depth = max_depth;
        self.authority = *authority;
        self.creation_slot = creation_slot;
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct MerkleRollPreAppendHeader {
    /// Depth of associated merkle tree
    pub max_depth: u32,

    /// The next depth of tree which needs to be initialized  
    pub next_index_to_initialize: u16,

    /// Whether or not the merkle roll pre append header has been fully initialized
    pub initialized: u16
}

impl MerkleRollPreAppendHeader {
    pub fn initialize(
        &mut self,
        max_depth: u32
    ) {
        self.max_depth = max_depth;
        self.next_index_to_initialize = 0;
        self.initialized = 0;
    }
}

#[derive(Clone)]
pub struct CandyWrapper;

impl anchor_lang::Id for CandyWrapper {
    fn id() -> Pubkey {
        candy_wrapper::id()
    }
}
