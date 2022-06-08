use anchor_lang::prelude::*;

#[error_code]
pub enum BubblegumError {
    #[msg("Missing authority bump seed")]
    MissingAuthorityBumpSeed,
    #[msg("Master edition supply expected to be 0")]
    MasterEditionSupplyNonzero,
    #[msg("Compression ineligible for the TokenProgram that this NFT was made with")]
    CompressTokenProgramIneligible,
}
