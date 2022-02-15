use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, CloseAccount, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::{
    instruction::{
        create_master_edition_v3, create_metadata_accounts_v2, update_metadata_accounts_v2,
    },
    state::{Metadata, MAX_METADATA_LEN},
    utils::try_from_slice_checked,
};
use solana_program::{hash::hashv, program::invoke_signed, sysvar};
use spl_token_2022::instruction::close_account;
use std::io::Write;

const MERKLE_PREFIX: &str = "MERKLE";

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
pub mod merkle_wallet {
    use super::*;
    pub fn initialize_merkle_wallet(
        ctx: Context<InitializeMerkleWallet>,
        bump: u8,
        airdrop_bump: u8,
    ) -> ProgramResult {
        let mut merkle_wallet = ctx.accounts.merkle_wallet.load_init()?;
        merkle_wallet.root = EMPTY;
        merkle_wallet.counter = 0;
        merkle_wallet.bump = bump as u64;
        let mut merkle_airdrop_wallet = ctx.accounts.merkle_airdrop_wallet.load_init()?;
        merkle_airdrop_wallet.root = EMPTY;
        merkle_airdrop_wallet.bump = airdrop_bump as u64;
        Ok(())
    }

    pub fn mint_nft<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, MintNFT<'info>>,
        authority_bump: u8,
    ) -> ProgramResult {
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.authority.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                },
                &[&[MERKLE_PREFIX.as_ref(), &[authority_bump as u8]]],
            ),
            1,
        )?;
        let mut merkle_wallet = ctx.accounts.merkle_wallet.load_mut()?;
        merkle_wallet.counter += 1;
        Ok(())
    }

    pub fn compress_nft(
        ctx: Context<CompressNFT>,
        authority_bump: u8,
        params: CompressNFTArgs,
    ) -> ProgramResult {
        let merkle_wallet = ctx.accounts.merkle_wallet.load()?;
        assert_with_msg(
            recompute(EMPTY, params.proof.as_ref(), params.path) == merkle_wallet.root,
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
        // TODO: This should probably be a CPI into the Token Metadata program
        // By this point the mint authority should have changed to a PDA of the
        // Token Metadata Program
        invoke_signed(
            &close_account(
                &ctx.accounts.token_program.key(),
                &ctx.accounts.mint.key(),
                &ctx.accounts.owner.key(),
                &ctx.accounts.authority.key(),
                &[],
            )?,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.authority.to_account_info(),
            ],
            &[&[MERKLE_PREFIX.as_ref(), &[authority_bump as u8]]],
        )?;
        let (metadata_key, _) = Pubkey::find_program_address(
            &[
                mpl_token_metadata::state::PREFIX.as_ref(),
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
        let (master_edition_key, _) = Pubkey::find_program_address(
            &[
                mpl_token_metadata::state::PREFIX.as_ref(),
                mpl_token_metadata::id().as_ref(),
                ctx.accounts.mint.key().as_ref(),
                mpl_token_metadata::state::EDITION.as_ref(),
            ],
            &mpl_token_metadata::id(),
        );
        assert_with_msg(
            master_edition_key == ctx.accounts.master_edition.key(),
            ProgramError::InvalidArgument,
            "Master Edition key derivation failed",
        )?;
        // TODO: Destroy Metadata Account (Add instruction to Metaplex)
        let leaf = generate_leaf_node(&[
            &ctx.accounts.metadata.try_borrow_mut_data()?.as_ref(),
            &ctx.accounts.master_edition.try_borrow_mut_data()?.as_ref(),
            &ctx.accounts.mint_creator.key().as_ref(),
            &params.index.to_le_bytes().as_ref(),
        ])?;
        let new_root = recompute(leaf, params.proof.as_ref(), params.path);
        let mut merkle_wallet = ctx.accounts.merkle_wallet.load_mut()?;
        merkle_wallet.root = new_root;
        Ok(())
    }

    pub fn decompress_nft(
        ctx: Context<DecompressNFT>,
        _mint_bump: u8,
        authority_bump: u8,
        params: DecompressNFTArgs,
    ) -> ProgramResult {
        let merkle_wallet = ctx.accounts.merkle_wallet.load()?;
        let mut metadata = Box::new(vec![]);
        params.metadata.serialize(&mut metadata)?;
        let mut master_edition = Box::new(vec![]);
        // params.master_edition.serialize(&mut master_edition)?;
        let leaf = generate_leaf_node(&[
            metadata.as_ref(),
            master_edition.as_ref(),
            ctx.accounts.mint_creator.key().as_ref(),
            params.index.to_le_bytes().as_ref(),
        ])?;
        assert_with_msg(
            recompute(leaf, params.proof.as_ref(), params.path) == merkle_wallet.root,
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
                &[&[MERKLE_PREFIX.as_ref(), &[authority_bump]]],
            ),
            1,
        )?;
        // TODO: Restore Metadata Account (Add Metaplex instructions)
        let creators = match params.metadata.data.creators {
            Some(c) => c,
            None => vec![],
        };

        let metadata_infos = vec![
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ];

        let master_edition_infos = vec![
            ctx.accounts.master_edition.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ];

        invoke_signed(
            &create_metadata_accounts_v2(
                *ctx.accounts.token_metadata_program.key,
                ctx.accounts.metadata.key(),
                ctx.accounts.mint.key(),
                ctx.accounts.authority.key(),
                ctx.accounts.owner.key(),
                ctx.accounts.authority.key(),
                params.metadata.data.name,
                params.metadata.data.symbol.clone(),
                params.metadata.data.uri,
                Some(creators),
                params.metadata.data.seller_fee_basis_points,
                true,
                params.metadata.is_mutable,
                params.metadata.collection,
                params.metadata.uses,
            ),
            metadata_infos.as_slice(),
            &[&[MERKLE_PREFIX.as_ref(), &[authority_bump]]],
        )?;

        // msg!("Before master");
        // invoke_signed(
        //     &create_master_edition(
        //         ctx.accounts.token_metadata_program.key(),
        //         ctx.accounts.master_edition.key(),
        //         ctx.accounts.mint.key(),
        //         ctx.accounts.authority.key(),
        //         ctx.accounts.authority.key(),
        //         ctx.accounts.metadata.key(),
        //         ctx.accounts.owner.key(),
        //         Some(params.metadata.data.max_supply),
        //     ),
        //     master_edition_infos.as_slice(),
        //     &[&[MERKLE_PREFIX.as_ref(), &[authority_bump]]],
        // )?;

        // msg!("Before update");
        // invoke_signed(
        //     &update_metadata_accounts(
        //         ctx.accounts.token_metadata_program.key(),
        //         ctx.accounts.metadata.key(),
        //         ctx.accounts.authority.key(),
        //         new_update_authority,
        //         None,
        //         Some(true),
        //     ),
        //     &[
        //         ctx.accounts.token_metadata_program.to_account_info(),
        //         ctx.accounts.metadata.to_account_info(),
        //         ctx.accounts.authority.to_account_info(),
        //     ],
        //     &[&[MERKLE_PREFIX.as_ref(), &[authority_bump]]],
        // )?;
        let mut merkle_wallet = ctx.accounts.merkle_wallet.load_mut()?;
        merkle_wallet.root = recompute(EMPTY, &params.proof, params.path);
        Ok(())
    }
}

fn generate_leaf_node<'info>(seeds: &[&[u8]]) -> Result<Node, ProgramError> {
    let mut leaf = EMPTY;
    for seed in seeds.iter() {
        let hash = hashv(&[&leaf, seed]);
        leaf.copy_from_slice(hash.as_ref());
    }
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
pub struct MerkleWallet {
    root: Node,
    bump: u64,
    counter: u128,
}

#[account(zero_copy)]
pub struct AirdropMerkleWallet {
    root: Node,
    bump: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CompressNFTArgs {
    index: u128,
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
pub struct InitializeMerkleWallet<'info> {
    #[account(
        init,
        seeds = [
            MERKLE_PREFIX.as_ref(),
            payer.key().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + 32 + 16 + 8,
    )]
    pub merkle_wallet: AccountLoader<'info, MerkleWallet>,
    #[account(
        init,
        seeds = [
            MERKLE_PREFIX.as_ref(),
            b"airdrop".as_ref(),
            payer.key().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + 32 + 16 + 8,
    )]
    pub merkle_airdrop_wallet: AccountLoader<'info, AirdropMerkleWallet>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(authority_bump: u8)]
pub struct MintNFT<'info> {
    #[account(
        mut,
        seeds = [
            MERKLE_PREFIX.as_ref(),
            owner.key().as_ref(),
        ],
        bump,
    )]
    pub merkle_wallet: AccountLoader<'info, MerkleWallet>,
    #[account(
        init,
        seeds = [
            owner.key().as_ref(),
            merkle_wallet.load()?.counter.to_le_bytes().as_ref(),
        ],
        bump,
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
    #[account(
        seeds = [MERKLE_PREFIX.as_ref()],
        bump = authority_bump,
    )]
    pub authority: AccountInfo<'info>,
    pub owner: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(authority_bump: u8, params: CompressNFTArgs)]
pub struct CompressNFT<'info> {
    #[account(
        mut,
        seeds = [
            MERKLE_PREFIX.as_ref(),
            owner.key().as_ref(),
        ],
        bump = merkle_wallet.load()?.bump as u8,
    )]
    pub merkle_wallet: AccountLoader<'info, MerkleWallet>,
    #[account(
        mut,
        constraint = token_account.mint == mint.key(),
        constraint = token_account.amount == 1,
    )]
    pub token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            mint_creator.key().as_ref(),
            params.index.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub mint: Account<'info, Mint>,
    pub mint_creator: AccountInfo<'info>,
    #[account(mut)]
    pub metadata: AccountInfo<'info>,
    #[account(mut)]
    pub master_edition: AccountInfo<'info>,
    pub owner: Signer<'info>,
    #[account(
        seeds = [
            MERKLE_PREFIX.as_ref()
        ],
        bump = authority_bump
    )]
    pub authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(authority_bump: u8, params: DecompressNFTArgs)]
pub struct DecompressNFT<'info> {
    #[account(
        mut,
        seeds = [
            MERKLE_PREFIX.as_ref(),
            owner.key().as_ref(),
        ],
        bump = merkle_wallet.load()?.bump as u8,
    )]
    pub merkle_wallet: AccountLoader<'info, MerkleWallet>,
    #[account(
        init,
        seeds = [
            mint_creator.key().as_ref(),
            params.index.to_le_bytes().as_ref(),
        ],
        bump,
        payer = owner,
        space = Mint::LEN,
        mint::decimals = 1,
        mint::authority = authority,
    )]
    pub mint: Account<'info, Mint>,
    pub mint_creator: AccountInfo<'info>,
    #[account(
        init,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = owner,
    )]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub metadata: AccountInfo<'info>,
    #[account(mut)]
    pub master_edition: AccountInfo<'info>,
    #[account(
        seeds = [
            MERKLE_PREFIX.as_ref(),
        ],
        bump = authority_bump,
    )]
    pub authority: AccountInfo<'info>,
    pub owner: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    #[account(
        executable,
        address = mpl_token_metadata::id(),
    )]
    pub token_metadata_program: AccountInfo<'info>,
}
