use anchor_lang::{prelude::*, solana_program::keccak};
use gummyroll::{program::Gummyroll, Node};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[derive(Accounts)]
#[instruction(max_depth: u32, max_buffer_size: u32)]
pub struct CreateTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"gummyroll-crud-authority-pda",
            merkle_roll.key().as_ref(),
            authority.key().as_ref(),
        ],
        bump,
        payer = authority,
        space = 0
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority_pda: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Add<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [
            b"gummyroll-crud-authority-pda",
            merkle_roll.key().as_ref(),
            authority.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority_pda: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Remove<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [
            b"gummyroll-crud-authority-pda",
            merkle_roll.key().as_ref(),
            authority.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority_pda: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    #[account(
        seeds = [
            b"gummyroll-crud-authority-pda",
            merkle_roll.key().as_ref(),
            authority.key().as_ref(),
        ],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority_pda: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub new_owner: UncheckedAccount<'info>,
}

#[program]
pub mod gummyroll_crud {

    use super::*;

    pub fn create_tree(
        ctx: Context<CreateTree>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let authority = ctx.accounts.authority.to_account_info();
        let authority_pda = ctx.accounts.authority_pda.to_account_info();
        let authority_pda_bump_seed = &[*ctx.bumps.get("authority_pda").unwrap()];
        let seeds = &[
            b"gummyroll-crud-authority-pda",
            merkle_roll.key.as_ref(),
            authority.key.as_ref(),
            authority_pda_bump_seed,
        ];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            gummyroll_program,
            gummyroll::cpi::accounts::Initialize {
                authority: authority_pda.clone(),
                merkle_roll,
            },
            authority_pda_signer,
        );
        gummyroll::cpi::init_empty_gummyroll(cpi_ctx, max_depth, max_buffer_size)
    }

    pub fn add(ctx: Context<Add>, message: Vec<u8>) -> Result<()> {
        let authority = ctx.accounts.authority.to_account_info();
        let authority_pda = ctx.accounts.authority_pda.to_account_info();
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let authority_pda_bump_seed = &[*ctx.bumps.get("authority_pda").unwrap()];
        let seeds = &[
            b"gummyroll-crud-authority-pda",
            merkle_roll.key.as_ref(),
            authority.key.as_ref(),
            authority_pda_bump_seed,
        ];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            gummyroll_program,
            gummyroll::cpi::accounts::Modify {
                authority: authority_pda.clone(),
                merkle_roll,
            },
            authority_pda_signer,
        );
        let leaf = Node::new(get_message_hash(&authority, &message).to_bytes());
        gummyroll::cpi::append(cpi_ctx, leaf)
    }

    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, Transfer<'info>>,
        root: [u8; 32],
        message: Vec<u8>,
        index: u32,
    ) -> Result<()> {
        let authority = ctx.accounts.authority.to_account_info();
        let authority_pda = ctx.accounts.authority_pda.to_account_info();
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let owner = ctx.accounts.owner.to_account_info();
        let new_owner = ctx.accounts.new_owner.to_account_info();
        let authority_pda_bump_seed = &[*ctx.bumps.get("authority_pda").unwrap()];
        let seeds = &[
            b"gummyroll-crud-authority-pda",
            merkle_roll.key.as_ref(),
            authority.key.as_ref(),
            authority_pda_bump_seed,
        ];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            gummyroll_program,
            gummyroll::cpi::accounts::Modify {
                authority: authority_pda.clone(),
                merkle_roll,
            },
            authority_pda_signer,
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
        leaf_hash: [u8; 32],
        index: u32,
    ) -> Result<()> {
        let authority = ctx.accounts.authority.to_account_info();
        let authority_pda = ctx.accounts.authority_pda.to_account_info();
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let authority_pda_bump_seed = &[*ctx.bumps.get("authority_pda").unwrap()];
        let seeds = &[
            b"gummyroll-crud-authority-pda",
            merkle_roll.key.as_ref(),
            authority.key.as_ref(),
            authority_pda_bump_seed,
        ];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            gummyroll_program,
            gummyroll::cpi::accounts::Modify {
                authority: authority_pda.clone(),
                merkle_roll,
            },
            authority_pda_signer,
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());

        let previous_leaf_node = Node::new(leaf_hash);
        let leaf_node = Node::default();
        let root_node = Node::new(root);
        gummyroll::cpi::replace_leaf(cpi_ctx, root_node, previous_leaf_node, leaf_node, index)
    }
}

pub fn get_message_hash(owner: &AccountInfo, message: &Vec<u8>) -> keccak::Hash {
    keccak::hashv(&[&owner.key().to_bytes(), message.as_slice()])
}
