use crate::utils::error_msg;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct GumballMachineHeader {
    // TODO: Add more fields
    pub url_base: [u8; 64],
    pub name_base: [u8; 32],
    pub symbol: [u8; 32],
    pub seller_fee_basis_points: u16,
    pub is_mutable: u8,
    pub retain_authority: u8,
    pub use_method: u8,
    pub _padding: [u8; 3],
    pub use_method_remaining: u64,
    pub use_method_total: u64,
    pub price: u64,
    pub go_live_date: i64,
    pub mint: Pubkey,
    pub bot_wallet: Pubkey,
    pub authority: Pubkey,
    pub collection_key: Pubkey,
    // Force a single creator (use Hydra)
    pub creator_address: Pubkey,
    pub extension_len: usize,
    pub remaining: usize,
    pub max_items: u64,
    pub total_items_added: usize,
}

impl ZeroCopy for GumballMachineHeader {}
pub trait ZeroCopy: Pod {
    fn load_mut_bytes<'a>(data: &'a mut [u8]) -> Result<&'a mut Self> {
        let size = size_of::<Self>();
        let data_len = data.len();

        Ok(bytemuck::try_from_bytes_mut(&mut data[..size])
            .map_err(error_msg::<Self>(data_len))
            .unwrap())
    }
}
