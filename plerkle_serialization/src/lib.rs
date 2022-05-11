pub mod account_info_generated;
pub mod block_info_generated;
pub mod slot_status_info_generated;
pub mod transaction_info_generated;

// Re-export plerkle_serialized at crate root level for
// easier access.
mod plerkle_serialized;
pub use plerkle_serialized::*;
