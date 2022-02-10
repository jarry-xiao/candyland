use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, CloseAccount, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::state::Metadata;
use solana_program::{hash::hashv, program::invoke_signed};
use spl_token_2022::instruction::close_account;

declare_id!("EfYFFrDJCyP7P8LSmHykAqdyJpPsJsxChFEPy5AJ5mR7");

#[inline(always)]
pub fn assert_with_msg(v: bool, err: ProgramError, msg: &str) -> ProgramResult {
    if !v {
        let caller = std::panic::Location::caller();
        msg!("{}. \n{}", msg, caller);
        Err(err.into())
    } else {
        Ok(())
    }
}

type Node = [u8; 32];
const EMPTY: Node = [0; 32];

#[program]
pub mod fractal {
    use super::*;
    pub fn initialize_collection(ctx: Context<Initialize>, data: Node) -> ProgramResult {
        let mut collection = ctx.accounts.collection.load_init()?;
        collection.root = data;
        // This will be the bump of the collection authority PDA
        let (_, bump) =
            Pubkey::find_program_address(&[ctx.accounts.collection.key().as_ref()], ctx.program_id);
        collection.bump = bump as u64;
        Ok(())
    }

    pub fn compress_nft(ctx: Context<CompressNFT>, params: CompressNFTArgs) -> ProgramResult {
        let collection = ctx.accounts.collection.load()?;
        assert_with_msg(
            recompute(EMPTY, params.proof.as_ref(), params.path) == collection.root,
            ProgramError::InvalidArgument,
            "Invalid Merkle proof provided",
        )?;
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            1,
        )?;
        token::close_account(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.token_account.to_account_info(),
                destination: ctx.accounts.owner.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ))?;
        // TODO: Close mint account (Token 2022)
        let (metadata_key, _) = Pubkey::find_program_address(
            &[
                b"metadata",
                mpl_token_metadata::id().as_ref(),
                ctx.accounts.mint.key().as_ref(),
            ],
            &mpl_token_metadata::id(),
        );
        assert_with_msg(
            metadata_key == ctx.accounts.metadata.key(),
            ProgramError::InvalidArgument,
            "Token metadata key derivation failed",
        )?;
        let leaf = generate_leaf_node(
            &ctx.accounts.metadata.try_borrow_mut_data()?.as_ref(),
            &ctx.accounts.collection,
            &ctx.accounts.owner.key(),
        )?;
        let new_root = recompute(leaf, params.proof.as_ref(), params.path);
        let mut collection = ctx.accounts.collection.load_mut()?;
        collection.root = new_root;
        collection.sequence_number += 1;
        Ok(())
    }

    pub fn decompress_nft(
        ctx: Context<DecompressNFT>,
        _bump: u8,
        params: DecompressNFTArgs,
    ) -> ProgramResult {
        let collection = ctx.accounts.collection.load()?;
        let mut metadata = Box::new(vec![]);
        params.metadata.serialize(&mut metadata)?;
        let leaf = generate_leaf_node(
            metadata.as_ref(),
            &ctx.accounts.collection,
            &ctx.accounts.owner.key(),
        )?;
        assert_with_msg(
            recompute(leaf, params.proof.as_ref(), params.path) == collection.root,
            ProgramError::InvalidArgument,
            "Invalid Merkle proof provided",
        )?;
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.authority.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                },
                &[&[
                    ctx.accounts.collection.key().as_ref(),
                    &[collection.bump as u8],
                ]],
            ),
            1,
        )?;
        // TODO: Restore Metadata Account (Add instruction to Metaplex)
        let mut collection = ctx.accounts.collection.load_mut()?;
        collection.root = recompute(EMPTY, &params.proof, params.path);
        Ok(())
    }
}

fn generate_leaf_node<'info>(
    metadata: &[u8],
    collection: &AccountLoader<'info, Collection>,
    owner: &Pubkey,
) -> Result<Node, ProgramError> {
    // leaf = hash(metadata, owner, collection, index)
    let mut leaf = EMPTY;
    let hash = hashv(&[&leaf, metadata]);
    leaf.copy_from_slice(hash.as_ref());
    let hash = hashv(&[&leaf, owner.as_ref()]);
    leaf.copy_from_slice(hash.as_ref());
    let hash = hashv(&[&leaf, collection.key().as_ref()]);
    leaf.copy_from_slice(hash.as_ref());
    let sequence_number = collection.load()?.sequence_number;
    let hash = hashv(&[&leaf, sequence_number.to_le_bytes().as_ref()]);
    leaf.copy_from_slice(hash.as_ref());
    Ok(leaf)
}

fn recompute(mut start: [u8; 32], path: &[[u8; 32]], address: u32) -> [u8; 32] {
    for (ix, s) in path.iter().enumerate() {
        if address >> ix & 1 == 1 {
            let res = hashv(&[&start, s.as_ref()]);
            start.copy_from_slice(res.as_ref());
        } else {
            let res = hashv(&[s.as_ref(), &start]);
            start.copy_from_slice(res.as_ref());
        }
    }
    start
}

#[account(zero_copy)]
pub struct Collection {
    root: Node,
    bump: u64,
    sequence_number: u128,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CompressNFTArgs {
    path: u32,
    proof: Vec<Node>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressNFTArgs {
    index: u128,
    metadata: Metadata,
    path: u32,
    proof: Vec<Node>,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + 32 + 16 + 8)]
    pub collection: AccountLoader<'info, Collection>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CompressNFT<'info> {
    #[account(mut)]
    pub collection: AccountLoader<'info, Collection>,
    #[account(
        mut,
        constraint = token_account.mint == mint.key(),
        constraint = token_account.amount == 1,
    )]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub metadata: AccountInfo<'info>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(bump: u8, params: DecompressNFTArgs)]
pub struct DecompressNFT<'info> {
    #[account(mut)]
    pub collection: AccountLoader<'info, Collection>,
    #[account(
        init,
        seeds = [
            collection.key().as_ref(),
            params.index.to_le_bytes().as_ref(),
        ],
        bump = bump,
        payer = owner,
        space = Mint::LEN,
        mint::decimals = 1,
        mint::authority = authority,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = owner,
    )]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub metadata: AccountInfo<'info>,
    #[account(
        seeds = [
            collection.key().as_ref(),
        ],
        bump = collection.load()?.bump as u8,
    )]
    pub authority: AccountInfo<'info>,
    pub owner: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
