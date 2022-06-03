use anchor_lang::prelude::*;

pub const GUMMYROLL_TREE_AUTHORITY_SIZE: usize = 264 + 8;
#[account]
pub struct GummyrollTreeAuthority {
    /// Pubkey of merkle roll that this manages
    pub tree_id: Pubkey,
    /// How many NFTs have been minted
    pub count: u64,
    /// Always able to transfer owner, delegate, modify append_allowlist
    pub owner: Pubkey,
    /// Always able to transfer delegate, modify append_allowlist
    pub delegate: Pubkey,
    /// Always able to append via bubblegum
    pub append_allowlist: [Pubkey; 5],
}
