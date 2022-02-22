use crate::state::{metaplex_anchor::*, spl_token_2022_anchor::*};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use mpl_token_metadata::{state::MAX_METADATA_LEN, utils::try_from_slice_checked};
use solana_program::{
    hash::hashv,
    program::{invoke, invoke_signed},
    system_instruction,
};

pub mod state;

const MERKLE_PREFIX: &str = "MERKLE";

declare_id!("7iNZXYZDn1127tRp1GSe3W3zGqGNdw16SiCwANNfTqXH");

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

const EMPTY: [u8; 32] = [0; 32];

#[account]
#[derive(Default)]
pub struct MerkleAuthority {
    bump: u8,
}

#[account]
#[derive(Default)]
pub struct MerkleWallet {
    root: [u8; 32],
    counter: u128,
    bump: u8,
}

#[program]
pub mod merkle_wallet {
    use super::*;
    pub fn initialize_merkle_wallet(ctx: Context<InitializeMerkleWallet>) -> ProgramResult {
        ctx.accounts.merkle_wallet.root = EMPTY;
        ctx.accounts.merkle_wallet.counter = 0;
        match ctx.bumps.get("merkle_wallet") {
            Some(b) => {
                ctx.accounts.merkle_wallet.bump = *b;
            }
            _ => {
                msg!("Bump seed missing from ctx");
                return Err(ProgramError::InvalidArgument);
            }
        }
        Ok(())
    }

    pub fn mint_nft<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, MintNFT<'info>>,
    ) -> ProgramResult {
        invoke(
            &spl_token_2022::instruction::initialize_mint2(
                &spl_token_2022::id(),
                &ctx.accounts.mint.key(),
                &ctx.accounts.payer.key(),
                Some(&ctx.accounts.payer.key()),
                0,
            )?,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.mint.to_account_info(),
            ],
        )?;
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &ctx.accounts.payer.key(),
                &ctx.accounts.payer.key(),
                &ctx.accounts.mint.key(),
                &ctx.accounts.token_program.key(),
            ),
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.token_account.to_account_info(),
            ],
        )?;
        invoke(
            &spl_token_2022::instruction::mint_to(
                &spl_token_2022::id(),
                &ctx.accounts.mint.key(),
                &ctx.accounts.token_account.key(),
                &ctx.accounts.payer.key(),
                &[],
                1,
            )?,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.token_account.to_account_info(),
            ],
        )?;
        ctx.accounts.merkle_wallet.counter += 1;
        Ok(())
    }

    pub fn compress_nft(ctx: Context<CompressNFT>, params: CompressNFTArgs) -> ProgramResult {
        assert_with_msg(
            recompute(EMPTY, params.proof.as_ref(), params.path) == ctx.accounts.merkle_wallet.root,
            ProgramError::InvalidArgument,
            "Invalid Merkle proof provided",
        )?;
        invoke(
            &spl_token_2022::instruction::burn(
                &spl_token_2022::id(),
                &ctx.accounts.token_account.key(),
                &ctx.accounts.mint.key(),
                &ctx.accounts.owner.key(),
                &[],
                1,
            )?,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.token_account.to_account_info(),
            ],
        )?;
        invoke(
            &spl_token_2022::instruction::close_account(
                &spl_token_2022::id(),
                &ctx.accounts.token_account.key(),
                &ctx.accounts.owner.key(),
                &ctx.accounts.owner.key(),
                &[],
            )?,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.token_account.to_account_info(),
            ],
        )?;
        let max_supply = match ctx.accounts.master_edition.max_supply {
            Some(s) => s,
            None => 0,
        };

        let leaf = generate_leaf_node(&[
            &ctx.accounts
                .metadata
                .to_account_info()
                .try_borrow_mut_data()?
                .as_ref(),
            &max_supply.to_le_bytes().as_ref(),
            &ctx.accounts.master_edition.supply.to_le_bytes().as_ref(),
            &params.mint_creator.as_ref(),
            &params.index.to_le_bytes().as_ref(),
        ])?;
        let new_root = recompute(leaf, params.proof.as_ref(), params.path);
        ctx.accounts.merkle_wallet.root = new_root;
        // TODO: This should probably be a CPI into the Token Metadata program
        // By this point the mint authority should have changed to a PDA of the
        // Token Metadata Program
        // TODO: Destroy Metadata Account (Add instruction to Metaplex)

        Ok(())
    }

    pub fn decompress_nft(
        ctx: Context<DecompressNFT>,
        _mint_bump: u8,
        authority_bump: u8,
        params: DecompressNFTArgs,
    ) -> ProgramResult {
        let leaf = generate_leaf_node(&[
            params.metadata_bytes.as_ref(),
            params.max_supply.to_le_bytes().as_ref(),
            params.supply.to_le_bytes().as_ref(),
            params.mint_creator.as_ref(),
            params.index.to_le_bytes().as_ref(),
        ])?;
        assert_with_msg(
            recompute(leaf, params.proof.as_ref(), params.path) == ctx.accounts.merkle_wallet.root,
            ProgramError::InvalidArgument,
            "Invalid Merkle proof provided",
        )?;
        invoke_signed(
            &spl_token_2022::instruction::mint_to(
                &spl_token_2022::id(),
                &ctx.accounts.mint.key(),
                &ctx.accounts.token_account.key(),
                &ctx.accounts.authority.key(),
                &[],
                1,
            )?,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.token_account.to_account_info(),
            ],
            &[&[MERKLE_PREFIX.as_ref(), &[authority_bump]]],
        )?;
        let metadata = Box::<TokenMetadata>::new(try_from_slice_checked(
            &params.metadata_bytes,
            mpl_token_metadata::state::Key::MetadataV1,
            MAX_METADATA_LEN,
        )?);
        let _creators = match &metadata.data.creators {
            Some(c) => c.clone(),
            None => vec![],
        };
        // TODO: Restore Metadata Account (Add Metaplex instructions)
        ctx.accounts.merkle_wallet.root = recompute(EMPTY, &params.proof, params.path);
        Ok(())
    }
}

fn generate_leaf_node<'info>(seeds: &[&[u8]]) -> Result<[u8; 32], ProgramError> {
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
    )]
    pub merkle_wallet: Box<Account<'info, MerkleWallet>>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintNFT<'info> {
    #[account(
        mut,
        seeds = [
            MERKLE_PREFIX.as_ref(),
            payer.key().as_ref(),
        ],
        bump,
    )]
    pub merkle_wallet: Box<Account<'info, MerkleWallet>>,
    #[account(
        init,
        seeds = [
            payer.key().as_ref(),
            merkle_wallet.counter.to_le_bytes().as_ref(),
        ],
        bump,
        owner = spl_token_2022::id(),
        payer = payer,
        space = Mint::LEN,
    )]
    pub mint: UncheckedAccount<'info>,
    #[account(mut)]
    pub token_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token2022> ,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>, 
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CompressNFTArgs {
    index: u128,
    mint_creator: Pubkey,
    path: u32,
    proof: Vec<[u8; 32]>,
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
        bump = merkle_wallet.bump,
    )]
    pub merkle_wallet: Box<Account<'info, MerkleWallet>>,
    #[account(
        mut,
        constraint = token_account.mint == mint.key(),
        constraint = token_account.amount == 1,
    )]
    pub token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [
            params.mint_creator.as_ref(),
            params.index.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub mint: Box<Account<'info, Mint>>,
    #[account(mut)]
    pub metadata: Box<Account<'info, TokenMetadata>>,
    #[account(mut)]
    pub master_edition: Box<Account<'info, MasterEdition>>,
    pub owner: Signer<'info>,
    #[account(
        seeds = [MERKLE_PREFIX.as_ref()],
        bump = authority.bump,
    )]
    pub authority: Account<'info, MerkleAuthority>,
    pub token_program: Program<'info, Token2022>,
    pub token_metadata_program: Program<'info, MplTokenMetadata>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressNFTArgs {
    index: u128,
    mint_creator: Pubkey,
    max_supply: u64,
    supply: u64,
    metadata_bytes: Vec<u8>,
    path: u32,
    proof: Vec<[u8; 32]>,
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
        bump = merkle_wallet.bump as u8,
    )]
    pub merkle_wallet: Box<Account<'info, MerkleWallet>>,
    #[account(
        init,
        seeds = [
            params.mint_creator.as_ref(),
            params.index.to_le_bytes().as_ref(),
        ],
        bump,
        payer = owner,
        space = Mint::LEN,
        mint::decimals = 1,
        mint::authority = authority,
    )]
    pub mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = owner,
    )]
    pub token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub metadata: Box<Account<'info, TokenMetadata>>,
    #[account(mut)]
    pub master_edition: Box<Account<'info, MasterEdition>>,
    #[account(
        seeds = [MERKLE_PREFIX.as_ref()],
        bump = authority.bump,
    )]
    pub authority: Account<'info, MerkleAuthority>,
    pub owner: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, MplTokenMetadata>,
}
