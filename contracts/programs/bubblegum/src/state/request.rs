use anchor_lang::prelude::*;

pub const MINT_AUTHORITY_REQUEST_SIZE: usize = 57;

#[account]
#[derive(Copy, Debug)]
pub struct MintAuthorityRequest {
    pub mint_authority: Pubkey,
    pub mint_capacity: u64,
    pub num_minted: u64,
    pub approved: u8,
}
