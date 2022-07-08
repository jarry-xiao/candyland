use anchor_lang::prelude::*;
use concurrent_merkle_tree::error::CMTError;

/// Errors related to misconfiguration or misuse of the Merkle tree
#[error_code]
pub enum GummyrollError {
    /// This error is currently not used.
    #[msg("Incorrect leaf length. Expected vec of 32 bytes")]
    IncorrectLeafLength,

    /// A modification to the tree was invalid and a changelog was not emitted.
    /// The proof may be invalid or out-of-date, or the provided leaf hash was invalid.
    #[msg("Concurrent merkle tree error")]
    ConcurrentMerkleTreeError,

    /// An issue was detected with loading the provided account data for this Gummyroll tree.
    #[msg("Issue zero copying concurrent merkle tree data")]
    ZeroCopyError,

    /// The Gummyroll tree supports two configuration parameters: `max_depth` and `max_buffer_size`.
    /// - `max_buffer_size` must be a power of 2; valid values are limited to `(8, 64, 256, 512, 1024, 2048)`
    /// - `max_depth` can be any integer between 14 and 30.
    #[msg("An unsupported max depth or max buffer size constant was provided")]
    MerkleRollConstantsError,

    /// When using Canopy, the stored byte length should a multiple of the node's byte length (32 bytes)
    #[msg("Expected a different byte length for the merkle roll canopy")]
    CanopyLengthMismatch,
}

impl From<&CMTError> for GummyrollError {
    fn from(_error: &CMTError) -> Self {
        GummyrollError::ConcurrentMerkleTreeError
    }
}
