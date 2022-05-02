use crate::utils::error_msg;
use anchor_lang::{
    prelude::*,
    solana_program::{keccak::hashv, log::sol_log_compute_units},
};
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use mpl_token_metadata::state::Creator;
use std::convert::AsRef;
use std::mem::size_of;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct GumballMachineHeader {
    // TODO: Add more fields 
    pub url_base: [u8; 32],
    pub name_base: [u8; 32],
    pub symbol: [u8; 32],
    pub seller_fee_basis_points: u16,
    pub is_mutable: bool,
    pub price: u64,
    pub retain_authority: bool,
    pub go_live_date: i64,
    pub mint: Pubkey,
    // Force a single creator (use Hydra)
    pub creator_address: Pubkey,
    pub items_available: u64,
    pub total_items: u64,
    pub available_items: u64,
}

#[derive(Copy, Clone)]
pub struct ConfigLine {
    // Arweave extensions are 32 bytes
    pub id: u64,
    pub extension: [u8; 32],
}

#[derive(Copy, Clone)]
pub struct GumballMachine<const SIZE: usize> {
    pub remaining: u64,
    pub config_lines: [ConfigLine; SIZE],
}

unsafe impl<const SIZE: usize> Zeroable for GumballMachine<SIZE> {}
unsafe impl<const SIZE: usize> Pod for GumballMachine<SIZE> {}
impl<const SIZE: usize> ZeroCopy for GumballMachine<SIZE> {}

pub trait ZeroCopy: Pod {
    fn load_mut_bytes<'a>(data: &'a mut [u8]) -> Result<&'a mut Self> {
        let size = size_of::<Self>();
        let data_len = data.len();

        Ok(bytemuck::try_from_bytes_mut(&mut data[..size])
            .map_err(error_msg::<Self>(data_len))
            .unwrap())
    }
}

impl<const SIZE: usize> GumballMachine<SIZE> {
    pub fn sample(&mut self, entropy: u64) {
        let i = entropy % self.remaining;
        (&mut self.config_lines).swap(i as usize, self.remaining as usize - 1);
        self.remaining -= 1;
    }
}