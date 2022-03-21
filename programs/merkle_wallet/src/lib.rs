use crate::state::{metaplex_anchor::*, spl_token_2022_anchor::*};
use anchor_lang::{
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult,
        keccak::hashv,
        program::{invoke, invoke_signed},
    },
};
use anchor_spl::associated_token::AssociatedToken;
use spl_token_2022::{
    extension::{ExtensionType, ExtensionType::MintCloseAuthority},
    state::Mint as Mint2022,
};

pub mod state;

const MERKLE_PREFIX: &str = "MERKLE";

declare_id!("HCrUkHVeMqhsDuMvLSQa6HFKdB2ypF27r2pL2tpfLPBq");

#[inline(always)]
pub fn assert_with_msg(v: bool, err: ProgramError, msg: &str) -> ProgramResult {
    if !v {
        let caller = std::panic::Location::caller();
        msg!("{}. \n{}", msg, caller);
        Err(err)
    } else {
        Ok(())
    }
}

const EMPTY: [u8; 32] = [0; 32];
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
            &spl_token_2022::instruction::initialize_mint_close_authority(
                &spl_token_2022::id(),
                &ctx.accounts.mint.key(),
                Some(&ctx.accounts.authority.key()),
            )?,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.authority.to_account_info(),
            ],
        )?;
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

    // pub fn mint_nft_rent_free<'a, 'b, 'c, 'info>(
    //     ctx: Context<'a, 'b, 'c, 'info, MintNFTRentFree<'info>>,
    //     metadata: mpl_token_metadata::state::DataV2,
    //     path: u32,
    //     proof: Vec<[u8; 32]>,
    // ) -> ProgramResult {
    //     assert_with_msg(
    //         recompute(EMPTY, proof.as_ref(), path) == ctx.accounts.merkle_wallet.root,
    //         ProgramError::InvalidArgument,
    //         "Invalid Merkle proof provided",
    //     )?;
    //     let leaf = generate_leaf_node(&[
    //         metadata.try_to_vec()?.as_ref(),
    //         &ctx.accounts.authority.key().as_ref(),
    //         &ctx.accounts.merkle_wallet.counter.to_le_bytes().as_ref(),
    //     ])?;
    //     let new_root = recompute(leaf, proof.as_ref(), path);
    //     ctx.accounts.merkle_wallet.root = new_root;
    //     ctx.accounts.merkle_wallet.counter += 1;
    //     Ok(())
    // }

    pub fn compress_nft(
        ctx: Context<CompressNFT>,
        index: u128,
        mint_creator: Pubkey,
        path: u32,
        proof: Vec<[u8; 32]>,
    ) -> ProgramResult {
        let bump = match ctx.bumps.get("authority") {
            Some(b) => *b,
            _ => {
                msg!("Bump seed missing from ctx");
                return Err(ProgramError::InvalidArgument);
            }
        };
        assert_with_msg(
            recompute(EMPTY, proof.as_ref(), path) == ctx.accounts.merkle_wallet.root,
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
        match ctx.accounts.master_edition.max_supply {
            Some(d) if d > 0 => {
                msg!("Master edition must have a max supply of 0");
                return Err(ProgramError::InvalidAccountData);
            }
            _ => {}
        };
        let leaf = generate_leaf_node(&[
            ctx.accounts
                .metadata
                .to_account_info()
                .try_borrow_mut_data()?
                .as_ref(),
            mint_creator.as_ref(),
            index.to_le_bytes().as_ref(),
        ])?;
        let new_root = recompute(leaf, proof.as_ref(), path);
        ctx.accounts.merkle_wallet.root = new_root;
        invoke_signed(
            &spl_token_2022::instruction::close_account(
                &spl_token_2022::id(),
                &ctx.accounts.mint.key(),
                &ctx.accounts.owner.key(),
                &ctx.accounts.authority.key(),
                &[],
            )?,
            &[
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
            &[&[MERKLE_PREFIX.as_ref(), &[bump]]],
        )?;
        invoke(
            &mpl_token_metadata::instruction::close_metadata_and_master_edition(
                mpl_token_metadata::id(),
                ctx.accounts.metadata.key(),
                ctx.accounts.master_edition.key(),
                ctx.accounts.mint.key(),
                ctx.accounts.owner.key(),
                ctx.accounts.owner.key(),
            ),
            &[
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.metadata.to_account_info(),
                ctx.accounts.master_edition.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
        )
    }

    pub fn decompress_nft(
        _ctx: Context<DecompressNFT>,
        _mint_bump: u8,
        _params: DecompressNFTArgs,
    ) -> ProgramResult {
        // TODO
        Ok(())
    }
}

fn generate_leaf_node<'info>(seeds: &[&[u8]]) -> Result<[u8; 32]> {
    let mut leaf = EMPTY;
    for seed in seeds.iter() {
        let hash = hashv(&[leaf.as_ref(), seed]);
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
    /// CHECK: unsafe
    #[account(
        init,
        seeds = [
            payer.key().as_ref(),
            merkle_wallet.counter.to_le_bytes().as_ref(),
        ],
        bump,
        owner = spl_token_2022::id(),
        payer = payer,
        space = ExtensionType::get_account_len::<Mint2022>(&[MintCloseAuthority]),
    )]
    pub mint: UncheckedAccount<'info>,
    /// CHECK: unsafe
    #[account(mut)]
    pub token_account: UncheckedAccount<'info>,
    /// CHECK: unsafe
    #[account(
        seeds = [MERKLE_PREFIX.as_ref()],
        bump,
    )]
    pub authority: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintNFTRentFree<'info> {
    #[account(
        mut,
        seeds = [
            MERKLE_PREFIX.as_ref(),
            authority.key().as_ref(),
        ],
        bump,
    )]
    pub merkle_wallet: Box<Account<'info, MerkleWallet>>,
    /// CHECK: unsafe
    #[account(
        seeds = [MERKLE_PREFIX.as_ref()],
        bump,
    )]
    pub authority: AccountInfo<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CompressNFTArgs {
    index: u128,
    mint_creator: Pubkey,
    path: u32,
    // proof: Vec<[u8; 32]>,
}

#[derive(Accounts)]
pub struct CompressNFT<'info> {
    #[account(
        mut,
        seeds = [
            MERKLE_PREFIX.as_ref(),
            owner.key().as_ref(),
        ],
        bump,
    )]
    pub merkle_wallet: Box<Account<'info, MerkleWallet>>,
    /// CHECK: unsafe
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: unsafe
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    /// CHECK: unsafe
    #[account(
        seeds = [MERKLE_PREFIX.as_ref()],
        bump,
    )]
    pub authority: AccountInfo<'info>,
    #[account(mut)]
    pub metadata: Box<Account<'info, TokenMetadata>>,
    #[account(mut)]
    pub master_edition: Box<Account<'info, MasterEdition>>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub token_metadata_program: Program<'info, MplTokenMetadata>,
    pub system_program: Program<'info, System>,
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
#[instruction(params: DecompressNFTArgs)]
pub struct DecompressNFT {}
