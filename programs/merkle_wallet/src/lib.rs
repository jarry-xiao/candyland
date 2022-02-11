use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, CloseAccount, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::state::Metadata;
use solana_program::{hash::hashv, program::invoke_signed, sysvar};
use spl_token_2022::instruction::close_account;

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
        root: Node,
    ) -> ProgramResult {
        let mut merkle_wallet = ctx.accounts.merkle_wallet.load_init()?;
        merkle_wallet.root = root;
        // This will be the bump of the user's global merkle wallet PDA
        merkle_wallet.bump = bump as u64;
        Ok(())
    }

    pub fn mint_nft<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, MintNFT<'info>>,
        _mint_bump: u8,
        authority_bump: u8,
        _params: MintNFTArgs,
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
        Ok(())
    }

    pub fn compress_nft(
        ctx: Context<CompressNFT>,
        _mint_bump: u8,
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
        // TODO: Destroy Metadata Account (Add instruction to Metaplex)
        let leaf = generate_leaf_node(&[
            &ctx.accounts.metadata.try_borrow_mut_data()?.as_ref(),
            &params.uuid.as_ref(),
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
        let leaf = generate_leaf_node(&[metadata.as_ref(), &params.uuid.as_ref()])?;
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
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct MintNFTArgs {
    uuid: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CompressNFTArgs {
    uuid: [u8; 32],
    path: u32,
    proof: Vec<Node>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressNFTArgs {
    uuid: [u8; 32],
    metadata: Metadata,
    path: u32,
    proof: Vec<Node>,
}

#[derive(Accounts)]
#[instruction(bump: u8, params: MintNFTArgs)]
pub struct InitializeMerkleWallet<'info> {
    #[account(
        init, 
        seeds = [
            MERKLE_PREFIX.as_ref(),
            payer.key().as_ref(),
        ],
        bump = bump,
        payer = payer,
        space = 8 + 32 + 16 + 8,
    )]
    pub merkle_wallet: AccountLoader<'info, MerkleWallet>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(mint_bump: u8, authority_bump: u8, params: MintNFTArgs)]
pub struct MintNFT<'info> {
    #[account(
        init,
        seeds = [
            params.uuid.as_ref(),
        ],
        bump = mint_bump,
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
#[instruction(mint_bump: u8, authority_bump: u8, params: MintNFTArgs)]
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
            params.uuid.as_ref(),
        ],
        bump = mint_bump,
    )]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub metadata: AccountInfo<'info>,
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
#[instruction(mint_bump: u8, authority_bump: u8, params: DecompressNFTArgs)]
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
            params.uuid.as_ref(),
        ],
        bump = mint_bump,
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
}
