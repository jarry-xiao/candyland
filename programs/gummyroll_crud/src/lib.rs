use anchor_lang::{prelude::*, solana_program::keccak};
use anchor_lang::solana_program::instruction::Instruction;
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

pub enum InstructionName {
    Unknown,
    Add,
    Transfer,
    Remove
}
pub fn get_instruction_type(full_bytes: &Vec<u8>) -> InstructionName {
    let disc: [u8; 8] = {
        let mut disc = [0; 8];
        disc.copy_from_slice(&full_bytes[..8]);
        disc
    };
    match disc {
        [163, 52, 200, 231, 140, 3, 69, 186] => InstructionName::Transfer,
        [199, 186, 9, 79, 96, 129, 24, 106] => InstructionName::Remove,
        [41, 249, 249, 146, 197, 111, 56, 181] => InstructionName::Add,
        _ => InstructionName::Unknown
    }
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

    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, Transfer<'info>>,
        root: [u8; 32],
        message: Vec<u8>,
        index: u32,
    ) -> Result<()> {
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let owner = ctx.accounts.owner.to_account_info();
        let new_owner = ctx.accounts.new_owner.to_account_info();
        let cpi_ctx = CpiContext::new(
            gummyroll_program,
            gummyroll::cpi::accounts::Modify {
                authority: owner.clone(),
                merkle_roll,
            },
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());
        // It's important to synthesize the previous leaf ourselves, rather than to
        // accept it as an arg, so that we can ensure the message hasn't been modified.
        let previous_leaf_node = Node::new(get_message_hash(&owner, &message).to_bytes());
        let leaf_node = Node::new(get_message_hash(&new_owner, &message).to_bytes());
        let root_node = Node::new(root);
        gummyroll::cpi::replace_leaf(cpi_ctx, root_node, previous_leaf_node, leaf_node, index)
    }

    pub fn remove<'info>(
        ctx: Context<'_, '_, '_, 'info, Remove<'info>>,
        root: [u8; 32],
        message: Vec<u8>,
        index: u32,
    ) -> Result<()> {
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let owner = ctx.accounts.owner.to_account_info();
        let cpi_ctx = CpiContext::new(
            gummyroll_program,
            gummyroll::cpi::accounts::Modify {
                authority: owner.clone(),
                merkle_roll,
            },
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());
        // It's important to synthesize the previous leaf ourselves, rather than to
        // accept it as an arg, so that we can ensure the message is correct.
        let previous_leaf_node = Node::new(get_message_hash(&owner, &message).to_bytes());
        let leaf_node = Node::default();
        let root_node = Node::new(root);
        gummyroll::cpi::replace_leaf(cpi_ctx, root_node, previous_leaf_node, leaf_node, index)
    }
}

pub fn get_message_hash(owner: &AccountInfo, message: &Vec<u8>) -> keccak::Hash {
    keccak::hashv(&[&owner.key().to_bytes(), message.as_slice()])
}
