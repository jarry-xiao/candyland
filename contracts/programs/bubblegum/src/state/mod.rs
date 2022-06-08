pub mod leaf_schema;
pub mod metaplex_adapter;
pub mod metaplex_anchor;
pub mod spl_token_2022_anchor;

use anchor_lang::prelude::*;
use leaf_schema::LeafSchema;

#[account]
#[derive(Copy)]
pub struct Nonce {
    pub count: u128,
}

#[account]
#[derive(Copy)]
pub struct Voucher {
    pub leaf_schema: LeafSchema,
    pub index: u32,
    pub merkle_slab: Pubkey,
}

impl Voucher {
    pub fn new(leaf_schema: LeafSchema, index: u32, merkle_slab: Pubkey) -> Self {
        Self {
            leaf_schema,
            index,
            merkle_slab,
        }
    }
}
