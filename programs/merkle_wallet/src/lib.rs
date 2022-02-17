use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, CloseAccount, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::{
    state::{MAX_MASTER_EDITION_LEN, MAX_METADATA_LEN},
    utils::try_from_slice_checked,
};
use solana_program::{hash::hashv, program::invoke_signed};
use spl_token_2022::instruction::close_account;
use std::ops::Deref;

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
        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    authority: ctx.accounts.owner.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                },
            ),
            1,
        )?;
        ctx.accounts.merkle_wallet.counter += 1;
        Ok(())
    }

    pub fn compress_nft(
        ctx: Context<CompressNFT>,
        authority_bump: u8,
        params: CompressNFTArgs,
    ) -> ProgramResult {
        assert_with_msg(
            recompute(EMPTY, params.proof.as_ref(), params.path) == ctx.accounts.merkle_wallet.root,
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
            owner.key().as_ref(),
        ],
        bump,
    )]
    pub merkle_wallet: Box<Account<'info, MerkleWallet>>,
    #[account(
        init,
        seeds = [
            owner.key().as_ref(),
            merkle_wallet.counter.to_le_bytes().as_ref(),
        ],
        bump,
        payer = owner,
        space = Mint::LEN,
        mint::decimals = 1,
        mint::authority = owner,
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
    pub owner: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
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
    pub token_program: Program<'info, Token>,
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
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_metadata_program: Program<'info, MplTokenMetadata>,
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize)]
pub struct MasterEdition(mpl_token_metadata::state::MasterEditionV2);

impl anchor_lang::AccountDeserialize for MasterEdition {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_checked(
            &buf,
            mpl_token_metadata::state::Key::MasterEditionV2,
            MAX_MASTER_EDITION_LEN,
        )
    }
}

impl anchor_lang::AccountSerialize for MasterEdition {}

impl anchor_lang::Owner for MasterEdition {
    fn owner() -> Pubkey {
        mpl_token_metadata::id()
    }
}

impl Deref for MasterEdition {
    type Target = mpl_token_metadata::state::MasterEditionV2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize)]
pub struct TokenMetadata(mpl_token_metadata::state::Metadata);

impl anchor_lang::AccountDeserialize for TokenMetadata {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_checked(
            buf,
            mpl_token_metadata::state::Key::MetadataV1,
            MAX_METADATA_LEN,
        )
    }
}

impl anchor_lang::AccountSerialize for TokenMetadata {}

impl anchor_lang::Owner for TokenMetadata {
    fn owner() -> Pubkey {
        mpl_token_metadata::id()
    }
}

impl Deref for TokenMetadata {
    type Target = mpl_token_metadata::state::Metadata;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct MplTokenMetadata;

impl anchor_lang::Id for MplTokenMetadata {
    fn id() -> Pubkey {
        mpl_token_metadata::id()
    }
}
