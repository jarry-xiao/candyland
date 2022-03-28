use anchor_lang::prelude::*;
use gummyroll::program::Gummyroll;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[derive(Accounts)]
pub struct Add<'info> {
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
}

#[derive(Accounts)]
pub struct Remove<'info> {
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
    /// CHECK: This is safe because we don't read from or write to this account.
    pub new_owner: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
}

#[program]
pub mod gummyroll_crud {

    use super::*;

    pub fn add(_ctx: Context<Add>, _message: Vec<u8>) -> Result<()> {
        Ok(())
    }

    pub fn transfer(
        _ctx: Context<Transfer>,
        _root: [u8; 32],
        _message: Vec<u8>,
        _proof: Vec<[u8; 32]>,
        _index: u32,
    ) -> Result<()> {
        Ok(())
    }

    pub fn remove(
        _ctx: Context<Remove>,
        _root: [u8; 32],
        _message: Vec<u8>,
        _proof: Vec<[u8; 32]>,
        _index: u32,
    ) -> Result<()> {
        Ok(())
    }
}
