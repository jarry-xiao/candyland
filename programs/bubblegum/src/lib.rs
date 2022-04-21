use anchor_lang::{prelude::*, solana_program::keccak};
use anchor_spl::token::Token;
use gummyroll::{program::Gummyroll, state::node::Node};

pub mod state;

use crate::state::MetadataArgs;

declare_id!("BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o");

pub struct RawLeafSchema {
    pub owner: Pubkey,
    pub delegate: Pubkey, // Defaults to owner
    pub nonce: u128,
    pub data: Vec<u8>,
}

impl RawLeafSchema {
    pub fn new(owner: Pubkey, delegate: Pubkey, nonce: u128, data: Vec<u8>) -> Self {
        Self {
            owner,
            delegate,
            nonce,
            data,
        }
    }

    pub fn to_node(&self) -> Node {
        let hashed_leaf = keccak::hashv(&[
            self.owner.as_ref(),
            self.delegate.as_ref(),
            self.nonce.to_le_bytes().as_ref(),
            keccak::hashv(&[self.data.as_slice()]).as_ref(),
        ])
        .to_bytes();
        Node::new(hashed_leaf)
    }
}

pub struct LeafSchema {
    pub owner: Pubkey,
    pub delegate: Pubkey, // Defaults to owner
    pub nonce: u128,
    pub data_hash: [u8; 32],
}

impl LeafSchema {
    pub fn new(owner: Pubkey, delegate: Pubkey, nonce: u128, data_hash: [u8; 32]) -> Self {
        Self {
            owner,
            delegate,
            nonce,
            data_hash,
        }
    }

    pub fn to_node(&self) -> Node {
        let hashed_leaf = keccak::hashv(&[
            self.owner.as_ref(),
            self.delegate.as_ref(),
            self.nonce.to_le_bytes().as_ref(),
            self.data_hash.as_ref(),
        ])
        .to_bytes();
        Node::new(hashed_leaf)
    }
}
#[account]
#[derive(Copy)]
pub struct Nonce {
    pub count: u128,
}
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
    pub mint_authority: UncheckedAccount<'info>,
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
pub struct Decompress<'info> {
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority_pda: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub token_program: Program<'info, Token>,
    /// CHECK: unsafe
    pub mint: UncheckedAccount<'info>,
    /// CHECK: unsafe
    pub token_account: UncheckedAccount<'info>,
    /// CHECK: unsafe
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK: unsafe
    pub token_metadata: UncheckedAccount<'info>,
    /// CHECK: unsafe
    pub master_edition: UncheckedAccount<'info>,
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
            authority_pda_signer 
        );
        gummyroll::cpi::init_empty_gummyroll(cpi_ctx, max_depth, max_buffer_size)
    }

    pub fn mint(ctx: Context<Mint>, message: MetadataArgs) -> Result<()> {
        let owner = ctx.accounts.owner.key();
        let delegate = ctx.accounts.delegate.key();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let nonce = &mut ctx.accounts.nonce;
        let seed = merkle_roll.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("authority").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.gummyroll_program.to_account_info(),
            gummyroll::cpi::accounts::Append {
                authority: ctx.accounts.authority.to_account_info(),
                append_authority: ctx.accounts.mint_authority.to_account_info(),
                merkle_roll,
            },
            authority_pda_signer,
        );

        let leaf = RawLeafSchema::new(owner, delegate, nonce.count, message.try_to_vec()?);
        nonce.count = nonce.count.saturating_add(1);
        gummyroll::cpi::append(cpi_ctx, leaf.to_node())
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
        let seed = merkle_roll.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("authority").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.gummyroll_program.to_account_info(),
            gummyroll::cpi::accounts::Modify {
                authority: ctx.accounts.authority.to_account_info(),
                merkle_roll: ctx.accounts.merkle_roll.to_account_info(),
            },
            authority_pda_signer,
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());
        let previous_leaf = LeafSchema::new(owner.key(), delegate.key(), nonce, data_hash);
        // New leafs are instantiated with no delegate
        let new_leaf = LeafSchema::new(new_owner, new_owner, nonce, data_hash);
        let root_node = Node::new(root);
        gummyroll::cpi::replace_leaf(
            cpi_ctx,
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
        let seed = merkle_roll.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("authority").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.gummyroll_program.to_account_info(),
            gummyroll::cpi::accounts::Modify {
                authority: ctx.accounts.authority.to_account_info(),
                merkle_roll: ctx.accounts.merkle_roll.to_account_info(),
            },
            authority_pda_signer,
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());
        let previous_leaf = LeafSchema::new(owner, previous_delegate, nonce, data_hash);
        let new_leaf = LeafSchema::new(owner, new_delegate, nonce, data_hash);
        let root_node = Node::new(root);
        gummyroll::cpi::replace_leaf(
            cpi_ctx,
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
        let delegate = ctx.accounts.owner.to_account_info();
        assert!(owner.is_signer || delegate.is_signer);
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let seed = merkle_roll.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("authority").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.gummyroll_program.to_account_info(),
            gummyroll::cpi::accounts::Modify {
                authority: ctx.accounts.authority.to_account_info(),
                merkle_roll,
            },
            authority_pda_signer,
        )
        .with_remaining_accounts(ctx.remaining_accounts.to_vec());
        let previous_leaf = LeafSchema::new(owner.key(), delegate.key(), nonce, data_hash);
        let leaf_node = Node::default();
        let root_node = Node::new(root);
        gummyroll::cpi::replace_leaf(
            cpi_ctx,
            root_node,
            previous_leaf.to_node(),
            leaf_node,
            index,
        )
    }

    // pub fn decompress_to_hash(
    //     ctx: Context<Decompress<'info>>,
    //     root: [u8; 32],
    //     index: u32,
    // ) -> Result<()> {
    //     Ok(())
    // }

    // pub fn decompress_to_accounts(
    //     ctx: Context<Decompress<'info>>,
    //     root: [u8; 32],
    //     index: u32,
    // ) -> Result<()> {
    //     Ok(())
    // }

    // pub fn compress_from_hash() -> Result<()> {
    //     Ok(())
    // }

    // pub fn compress_from_accounts() -> Result<()> {
    //     Ok(())
    // }
}

