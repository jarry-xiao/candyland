use anchor_lang::{prelude::*, solana_program::keccak};
use gummyroll::{program::Gummyroll, Node};

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

    pub fn add(ctx: Context<Add>, message: Vec<u8>) -> Result<()> {
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let owner = ctx.accounts.owner.to_account_info();
        let cpi_ctx = CpiContext::new(
            gummyroll_program,
            gummyroll::cpi::accounts::Modify {
                authority: owner.clone(),
                merkle_roll,
            },
        );
        let leaf = Node::new(get_message_hash(&owner, &message).to_bytes());
        gummyroll::cpi::append(cpi_ctx, leaf)
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

pub fn get_message_hash(owner: &AccountInfo, message: &Vec<u8>) -> keccak::Hash {
    keccak::hashv(&[&owner.key().to_bytes(), message.as_slice()])
}
