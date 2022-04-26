use anchor_lang::prelude::*;
use gummyroll::{program::Gummyroll, state::node::Node};

pub mod state;
pub mod utils;

use crate::state::{
    leaf_schema::{LeafSchema, RawLeafSchema},
    metaplex_adapter::MetadataArgs,
    metaplex_anchor::{MasterEdition, TokenMetadata},
    Nonce, Voucher,
};
use crate::utils::{append_leaf, insert_or_append_leaf, replace_leaf};

declare_id!("BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o");

#[derive(Accounts)]
pub struct InitNonce<'info> {
    #[account(
        init,
        seeds = [b"bubblegum"],
        payer = payer,
        space = 8 + 16,
        bump,
    )]
    pub nonce: Account<'info, Nonce>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateTree<'info> {
    pub tree_creator: Signer<'info>,
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(zero)]
    /// CHECK: This account must be all zeros
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Mint<'info> {
    /// CHECK: This account is neither written to nor read from.
    pub mint_authority: Signer<'info>,
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"bubblegum"],
        bump,
    )]
    pub nonce: Account<'info, Nonce>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    pub owner: Signer<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub delegate: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Burn<'info> {
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    /// CHECK: This account is checked in the instruction
    pub owner: UncheckedAccount<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: This account is checked in the instruction
    pub owner: UncheckedAccount<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub new_owner: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: This account is modified in the downstream program
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Delegate<'info> {
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub previous_delegate: UncheckedAccount<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub new_delegate: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: This account is modified in the downstream program
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(_root: [u8; 32], _data_hash: [u8; 32], nonce: u128, _index: u32)]
pub struct Redeem<'info> {
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    #[account(
        init,
        seeds = [merkle_roll.key().as_ref(), nonce.to_le_bytes().as_ref()],
        payer = owner,
        space = 8 + 32 + 32 + 16 + 32 + 4 + 32,
        bump
    )]
    pub voucher: Account<'info, Voucher>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelRedeem<'info> {
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    #[account(
        mut,
        close = owner,
        seeds = [merkle_roll.key().as_ref(), voucher.leaf_schema.nonce.to_le_bytes().as_ref()],
        bump
    )]
    pub voucher: Account<'info, Voucher>,
    #[account(mut)]
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct Decompress<'info> {
    /// CHECK: This account is not read
    pub merkle_roll: UncheckedAccount<'info>,
    /// CHECK: This account is checked in the instruction
    pub owner: UncheckedAccount<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    #[account(
        mut,
        close = payer,
        seeds = [voucher.merkle_roll.as_ref(), voucher.leaf_schema.nonce.to_le_bytes().as_ref()],
        bump
    )]
    pub voucher: Account<'info, Voucher>,
    /// CHECK: versioning is handled in the instruction
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: versioning is handled in the instruction
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub metadata: Box<Account<'info, TokenMetadata>>,
    #[account(mut)]
    pub master_edition: Box<Account<'info, MasterEdition>>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK: versioning is handled in the instruction
    pub token_program: UncheckedAccount<'info>,
    /// CHECK:
    pub associated_token_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Compress<'info> {
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: This account is not read
    pub merkle_roll: UncheckedAccount<'info>,
    /// CHECK: This account is checked in the instruction
    pub owner: Signer<'info>,
    /// CHECK: This account is chekced in the instruction
    pub delegate: UncheckedAccount<'info>,
    /// CHECK: versioning is handled in the instruction
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: versioning is handled in the instruction
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub metadata: Box<Account<'info, TokenMetadata>>,
    #[account(mut)]
    pub master_edition: Box<Account<'info, MasterEdition>>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
}

#[program]
pub mod bubblegum {
    use super::*;

    pub fn initialize_nonce(_ctx: Context<InitNonce>) -> Result<()> {
        Ok(())
    }

    pub fn create_tree(
        ctx: Context<CreateTree>,
        max_depth: u32,
        max_buffer_size: u32,
    ) -> Result<()> {
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let seed = merkle_roll.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("authority").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.gummyroll_program.to_account_info(),
            gummyroll::cpi::accounts::Initialize {
                authority: ctx.accounts.authority.to_account_info(),
                append_authority: ctx.accounts.tree_creator.to_account_info(),
                merkle_roll,
            },
            authority_pda_signer,
        );
        gummyroll::cpi::init_empty_gummyroll(cpi_ctx, max_depth, max_buffer_size)
    }

    pub fn mint(ctx: Context<Mint>, message: MetadataArgs) -> Result<()> {
        let owner = ctx.accounts.owner.key();
        let delegate = ctx.accounts.delegate.key();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let nonce = &mut ctx.accounts.nonce;
        let leaf = RawLeafSchema::new(owner, delegate, nonce.count, message.try_to_vec()?);
        nonce.count = nonce.count.saturating_add(1);
        append_leaf(
            &merkle_roll.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            &ctx.accounts.merkle_roll.to_account_info(),
            leaf.to_node(),
        )
    }

    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, Transfer<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        nonce: u128,
        index: u32,
    ) -> Result<()> {
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let owner = ctx.accounts.owner.to_account_info();
        let delegate = ctx.accounts.delegate.to_account_info();
        // Transfers must be initiated either by the leaf owner or leaf delegate
        assert!(owner.is_signer || delegate.is_signer);
        let new_owner = ctx.accounts.new_owner.key();
        let previous_leaf = LeafSchema::new(owner.key(), delegate.key(), nonce, data_hash);
        // New leafs are instantiated with no delegate
        let new_leaf = LeafSchema::new(new_owner, new_owner, nonce, data_hash);
        let root_node = Node::new(root);
        replace_leaf(
            &merkle_roll.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_roll.to_account_info(),
            ctx.remaining_accounts,
            root_node,
            previous_leaf.to_node(),
            new_leaf.to_node(),
            index,
        )
    }

    pub fn delegate<'info>(
        ctx: Context<'_, '_, '_, 'info, Delegate<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        nonce: u128,
        index: u32,
    ) -> Result<()> {
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let owner = ctx.accounts.owner.key();
        let previous_delegate = ctx.accounts.previous_delegate.key();
        let new_delegate = ctx.accounts.new_delegate.key();
        let previous_leaf = LeafSchema::new(owner, previous_delegate, nonce, data_hash);
        let new_leaf = LeafSchema::new(owner, new_delegate, nonce, data_hash);
        let root_node = Node::new(root);
        replace_leaf(
            &merkle_roll.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_roll.to_account_info(),
            ctx.remaining_accounts,
            root_node,
            previous_leaf.to_node(),
            new_leaf.to_node(),
            index,
        )
    }

    pub fn burn<'info>(
        ctx: Context<'_, '_, '_, 'info, Burn<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        nonce: u128,
        index: u32,
    ) -> Result<()> {
        let owner = ctx.accounts.owner.to_account_info();
        let delegate = ctx.accounts.delegate.to_account_info();
        assert!(owner.is_signer || delegate.is_signer);
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let previous_leaf = LeafSchema::new(owner.key(), delegate.key(), nonce, data_hash);
        let new_leaf = Node::default();
        let root_node = Node::new(root);
        replace_leaf(
            &merkle_roll.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_roll.to_account_info(),
            ctx.remaining_accounts,
            root_node,
            previous_leaf.to_node(),
            new_leaf,
            index,
        )
    }

    pub fn redeem<'info>(
        ctx: Context<'_, '_, '_, 'info, Redeem<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        nonce: u128,
        index: u32,
    ) -> Result<()> {
        let owner = ctx.accounts.owner.key();
        let delegate = ctx.accounts.delegate.key();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let previous_leaf = LeafSchema::new(owner, delegate, nonce, data_hash);
        let new_leaf = Node::default();
        let root_node = Node::new(root);
        replace_leaf(
            &merkle_roll.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_roll.to_account_info(),
            ctx.remaining_accounts,
            root_node,
            previous_leaf.to_node(),
            new_leaf,
            index,
        )?;
        ctx.accounts
            .voucher
            .set_inner(Voucher::new(previous_leaf, index, merkle_roll.key()));
        Ok(())
    }

    pub fn cancel_redeem<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelRedeem<'info>>,
        root: [u8; 32],
    ) -> Result<()> {
        let voucher = &ctx.accounts.voucher;
        assert_eq!(ctx.accounts.owner.key(), voucher.leaf_schema.owner);
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let root_node = Node::new(root);
        insert_or_append_leaf(
            &merkle_roll.key(),
            *ctx.bumps.get("authority").unwrap(),
            &ctx.accounts.gummyroll_program.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.merkle_roll.to_account_info(),
            ctx.remaining_accounts,
            root_node,
            voucher.leaf_schema.to_node(),
            voucher.index,
        )
    }

    pub fn decompress(_ctx: Context<Decompress>, _metadata: MetadataArgs) -> Result<()> {
        // TODO
        Ok(())
    }

    pub fn compress(_ctx: Context<Compress>) -> Result<()> {
        Ok(())
    }
}
