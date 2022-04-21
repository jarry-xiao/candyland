use anchor_lang::{prelude::*, solana_program::keccak};
use gummyroll::{program::Gummyroll, Node};

pub mod state;

use crate::state::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub struct LeafSchema {
    pub owner: Pubkey,
    pub delegate: Pubkey, // Defaults to owner
    pub nonce: u128,
    pub data: Vec<u8>,
}

impl LeafSchema {
    pub fn new(owner: Pubkey, delegate: Pubkey, nonce: u128, data: Vec<u8>) -> Self {
        Self {
            owner, delegate, nonce, data
        }
    }

    pub fn to_leaf(&self) -> Node {
        let hashed_leaf = hash(&[self.owner.as_ref(), self.delegate.as_ref(), self.nonce.to_le_bytes().as_ref(), self.data.as_slice()]);
        Node::new()
    }
}

#[derive(Accounts)]
pub struct InitNonce<'info> {
    #[account(
        init
        seeds = [b"bubblegum"],
        payer = payer,
        size = 8 + 16,
        bump,
    )]
    pub nonce: Account<'info, Nonce>,
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
    pub authority_pda: UncheckedAccount<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(zero)]
    /// CHECK: This account must be all zeros 
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Mint<'info> {
    pub authority: Signer<'info>,
    #[account(
        seeds = [merkle_roll.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority_pda: UncheckedAccount<'info>,
    #[account(
        mut
        seeds = [b"bubblegum"],
        bump,
    )]
    pub nonce: Account<'info, Nonce>
    pub owner: Signer<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Burn<'info> {
    #[account(
        seeds = [authority.key().as_ref()],
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
    #[account(
        seeds = [authority.key().as_ref()],
        bump,
    )]
    /// CHECK: This account is neither written to nor read from.
    pub authority_pda: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
    pub gummyroll_program: Program<'info, Gummyroll>,
    #[account(mut)]
    /// CHECK: unsafe
    pub merkle_roll: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
    /// CHECK: This account is neither written to nor read from.
    pub new_owner: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Decompress<'info> {
    #[account(
        seeds = [authority.key().as_ref()],
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
    pub mint: UncheckedAccount<'info>,
    pub token_account: UncheckedAccount<'info>,
    pub token_metadata_program: UncheckedAccount<'info>,
    pub token_metadata: UncheckedAccount<'info>,
    pub master_edition: UncheckedAccount<'info>,
}


#[account]
pub struct Nonce {
    pub count: u128,
}

#[program]
pub mod bubblegum {

    use super::*;

    pub fn initialize_nonce(ctx: Context<Nonce>) -> Result<()> {
        Ok(())
    }

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
        let seeds = &[authority.key.as_ref(), authority_pda_bump_seed];
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

    pub fn mint(ctx: Context<Add>, message: MetadataArgs) -> Result<()> {
        let authority = ctx.accounts.authority.to_account_info();
        let authority_pda = ctx.accounts.authority_pda.to_account_info();
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let authority_pda_bump_seed = &[*ctx.bumps.get("authority_pda").unwrap()];
        let seeds = &[authority.key.as_ref(), authority_pda_bump_seed];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            gummyroll_program,
            gummyroll::cpi::accounts::Modify {
                authority: authority_pda.clone(),
                merkle_roll,
            },
            authority_pda_signer,
        );
        let leaf = Node::new(get_message_hash(&authority, &message.try_to_vec()?).to_bytes());
        gummyroll::cpi::append(cpi_ctx, leaf)
    }

    pub fn transfer<'info>(
        ctx: Context<'_, '_, '_, 'info, Transfer<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        nonce: u128,
        index: u32,
    ) -> Result<()> {
        let authority = ctx.accounts.authority.to_account_info();
        let authority_pda = ctx.accounts.authority_pda.to_account_info();
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let owner = ctx.accounts.owner.to_account_info();
        let new_owner = ctx.accounts.new_owner.to_account_info();
        let authority_pda_bump_seed = &[*ctx.bumps.get("authority_pda").unwrap()];
        let seeds = &[authority.key.as_ref(), authority_pda_bump_seed];
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
        let previous_leaf_node =
            Node::new(hash(&[owner.key.as_ref(), &data_hash, &index.to_le_bytes()]).to_bytes());
        let leaf_node =
            Node::new(hash(&[new_owner.key.as_ref(), &data_hash, &index.to_le_bytes()]).to_bytes());
        let root_node = Node::new(root);
        gummyroll::cpi::replace_leaf(cpi_ctx, root_node, previous_leaf_node, leaf_node, index)
    }

    pub fn burn<'info>(
        ctx: Context<'_, '_, '_, 'info, Burn<'info>>,
        root: [u8; 32],
        data_hash: [u8; 32],
        nonce: u128,
        index: u32,
    ) -> Result<()> {
        let owner = ctx.accounts.owner.to_account_info();
        let authority = ctx.accounts.authority.to_account_info();
        let authority_pda = ctx.accounts.authority_pda.to_account_info();
        let gummyroll_program = ctx.accounts.gummyroll_program.to_account_info();
        let merkle_roll = ctx.accounts.merkle_roll.to_account_info();
        let authority_pda_bump_seed = &[*ctx.bumps.get("authority_pda").unwrap()];
        let seeds = &[authority.key.as_ref(), authority_pda_bump_seed];
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

        let previous_leaf_node =
            Node::new(hash(&[owner.key.as_ref(), &data_hash, &index.to_le_bytes()]).to_bytes());
        let leaf_node = Node::default();
        let root_node = Node::new(root);
        gummyroll::cpi::replace_leaf(cpi_ctx, root_node, previous_leaf_node, leaf_node, index)
    }

    pub fn decompress(
        ctx: Context<'_, '_, '_, 'info, Burn<'info>>,
        root: [u8; 32],

        index: u32,
    ) -> Result<()> {
        Ok(())
    }

    pub fn compress() -> Result<()> {
        Ok(())
    }
}

pub fn get_message_hash(owner: &AccountInfo, message: &Vec<u8>) -> keccak::Hash {
    keccak::hashv(&[&owner.key().to_bytes(), message.as_slice()])
}

pub fn hash(seeds: &[&[u8]]) -> keccak::Hash {
    keccak::hashv(seeds)
}
