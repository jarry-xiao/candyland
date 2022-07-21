use crate::error::BubblegumError;
use anchor_lang::prelude::*;

pub const MINT_REQUEST_SIZE: usize = 41 + 8;

#[account]
#[derive(Copy, Debug)]
pub struct MintRequest {
    pub mint_authority: Pubkey,
    pub mint_capacity: u64,
    pub approved: u8,
}

impl MintRequest {
    pub fn init(&mut self, mint_authority: &Pubkey, mint_capacity: u64) {
        self.mint_authority = *mint_authority;
        self.mint_capacity = mint_capacity;
        self.approved = 0;
    }

    pub fn set_approved(&mut self) {
        self.approved = 1;
    }

    pub fn is_approved(&self) -> bool {
        return self.approved == 1;
    }

    pub fn process_mint(&mut self) -> Result<()> {
        if !self.is_approved() {
            return Err(BubblegumError::MintRequestNotApproved.into());
        }
        if !self.has_mint_capacity(1) {
            return Err(BubblegumError::InsufficientMintCapacity.into());
        }

        self.mint_capacity = self.mint_capacity.saturating_sub(1);
        Ok(())
    }

    pub fn has_mint_capacity(&self, capacity: u64) -> bool {
        self.mint_capacity >= capacity
    }
}
