use anchor_lang::prelude::*;

#[error_code]
pub enum BubblegumError {
    #[msg("Asset Owner Does not match")]
    AssetOwnerMismatch,
    #[msg("PublicKeyMismatch")]
    PublicKeyMismatch,
    #[msg("Hashing Mismatch Within Leaf Schema")]
    HashingMismatch,
    #[msg("Unsupported Schema Version")]
    UnsupportedSchemaVersion,
    #[msg("Could not find append authority in append allowlist")]
    AppendAuthorityNotFound,
    #[msg("Append allowlist index out of bounds")]
    AppendAllowlistIndexOutOfBounds,
}
