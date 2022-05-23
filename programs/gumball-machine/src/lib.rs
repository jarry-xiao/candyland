use anchor_lang::{
    prelude::*,
    solana_program::{keccak::hashv, sysvar, sysvar::SysvarId},
};
use anchor_spl::token::Mint;
use bubblegum::program::Bubblegum;
use bubblegum::state::metaplex_adapter::UseMethod;
use bubblegum::state::metaplex_adapter::Uses;
use bytemuck::cast_slice_mut;
use gummyroll::program::Gummyroll;
pub mod state;
pub mod utils;

use crate::state::{GumballMachineHeader, ZeroCopy};
use crate::utils::get_metadata_args;

declare_id!("BRKyVDRGT7SPBtMhjHN4PVSPVYoc3Wa3QTyuRVM4iZkt");

#[derive(Accounts)]
pub struct InitGumballMachine<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(zero)]
    gumball_machine: AccountInfo<'info>,
    creator: Signer<'info>,
    mint: Account<'info, Mint>,
    /// CHECK: Mint/append authority to the merkle slab
    #[account(
        seeds = [gumball_machine.key().as_ref()],
        bump,
    )]
    willy_wonka: AccountInfo<'info>,
    /// CHECK: Tree authority to the merkle slab
    bubblegum_authority: AccountInfo<'info>,
    gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Empty merkle slab
    #[account(zero)]
    merkle_slab: AccountInfo<'info>,
    bubblegum: Program<'info, Bubblegum>,
}

#[derive(Accounts)]
pub struct UpdateConfigLine<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateConfigMetadata<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Dispense<'info> {
    /// CHECK: Validation occurs in instruction
    #[account(mut)]
    gumball_machine: AccountInfo<'info>,
    payer: Signer<'info>,
    #[account(
        seeds = [gumball_machine.key().as_ref()],
        bump,
    )]
    /// CHECK: Address is verified
    #[account(address = SlotHashes::id())]
    recent_blockhashes: UncheckedAccount<'info>,
    /// CHECK: Address is verified
    #[account(address = sysvar::instructions::id())]
    instruction_sysvar_account: UncheckedAccount<'info>,
    /// CHECK: PDA is checked on CPI for mint
    willy_wonka: AccountInfo<'info>,
    /// CHECK: PDA is checked in CPI from Bubblegum to Gummyroll
    /// This key must sign for all write operations to the NFT Metadata stored in the Merkle slab
    bubblegum_authority: AccountInfo<'info>,
    /// CHECK: PDA is checked in Bubblegum
    nonce: AccountInfo<'info>,
    gummyroll: Program<'info, Gummyroll>,
    /// CHECK: Validation occurs in Gummyroll
    merkle_slab: AccountInfo<'info>,
    bubblegum: Program<'info, Bubblegum>,
}

#[derive(Accounts)]
pub struct Destroy<'info> {
    /// CHECK: Validation occurs in instruction
    gumball_machine: AccountInfo<'info>,
    authority: Signer<'info>,
}

#[program]
pub mod gumball_machine {
    use super::*;

    pub fn initialize_gumball_machine(
        ctx: Context<InitGumballMachine>,
        max_depth: u32,
        max_buffer_size: u32,
        url_base: [u8; 64],
        name_base: [u8; 32],
        symbol: [u8; 32],
        seller_fee_basis_points: u16,
        is_mutable: bool,
        retain_authority: bool,
        price: u64,
        go_live_date: i64,
        bot_wallet: Pubkey,
        authority: Pubkey,
        collection_key: Pubkey,
        extension_len: u64,
        max_mint_size: u64,
        max_items: u64,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        let size = max_items as usize;
        *gumball_header = GumballMachineHeader {
            url_base: url_base,
            name_base: name_base,
            symbol: symbol,
            seller_fee_basis_points,
            is_mutable: is_mutable.into(),
            retain_authority: retain_authority.into(),
            _padding: [0; 4],
            price,
            go_live_date,
            bot_wallet,
            authority,
            mint: ctx.accounts.mint.key(),
            collection_key,
            creator_address: ctx.accounts.creator.key(),
            extension_len: extension_len as usize,
            max_mint_size: max_mint_size.max(1).min(max_items),
            remaining: 0,
            max_items,
            total_items_added: 0,
        };
        let index_array_size = std::mem::size_of::<u32>() * size;
        let config_size = extension_len as usize * size;
        assert!(config_data.len() == index_array_size + config_size);
        let (indices_data, _) = config_data.split_at_mut(index_array_size);
        let indices = cast_slice_mut::<u8, u32>(indices_data);
        indices
            .iter_mut()
            .enumerate()
            .for_each(|(i, idx)| *idx = i as u32);
        let seed = ctx.accounts.gumball_machine.key();
        let seeds = &[seed.as_ref(), &[*ctx.bumps.get("willy_wonka").unwrap()]];
        let authority_pda_signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.bubblegum.to_account_info(),
            bubblegum::cpi::accounts::CreateTree {
                tree_creator: ctx.accounts.willy_wonka.to_account_info(),
                authority: ctx.accounts.bubblegum_authority.to_account_info(),
                gummyroll_program: ctx.accounts.gummyroll.to_account_info(),
                merkle_slab: ctx.accounts.merkle_slab.to_account_info(),
            },
            authority_pda_signer,
        );
        bubblegum::cpi::create_tree(cpi_ctx, max_depth, max_buffer_size)
    }

    /// Add can only append config lines to the the end of the list
    pub fn add_config_lines(
        ctx: Context<UpdateConfigLine>,
        new_config_lines_data: Vec<u8>,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let mut gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        let size = gumball_header.max_items as usize;
        let index_array_size = std::mem::size_of::<u32>() * size;
        let config_size = gumball_header.extension_len * size;
        let line_size = gumball_header.extension_len;
        let num_lines = new_config_lines_data.len() / line_size;
        let start_index = gumball_header.total_items_added;
        assert_eq!(gumball_header.authority, ctx.accounts.authority.key());
        assert_eq!(new_config_lines_data.len() % line_size, 0);
        assert!(config_data.len() == index_array_size + config_size);
        assert_eq!(new_config_lines_data.len(), num_lines * line_size);
        assert!(start_index + num_lines <= gumball_header.max_items as usize);
        let (_, config_lines_data) = config_data.split_at_mut(index_array_size);
        config_lines_data[start_index..]
            .iter_mut()
            .take(num_lines)
            .enumerate()
            .for_each(|(i, l)| *l = new_config_lines_data[i]);
        gumball_header.total_items_added += num_lines;
        gumball_header.remaining += num_lines;
        Ok(())
    }

    /// Update only allows the authority to modify previously appended lines
    pub fn update_config_lines(
        ctx: Context<UpdateConfigLine>,
        starting_line: u64,
        new_config_lines_data: Vec<u8>,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        let size = gumball_header.max_items as usize;
        let index_array_size = std::mem::size_of::<u32>() * size;
        let config_size = gumball_header.extension_len * size;
        let line_size = gumball_header.extension_len;
        let num_lines = new_config_lines_data.len() / line_size;
        assert_eq!(gumball_header.authority, ctx.accounts.authority.key());
        assert_eq!(new_config_lines_data.len() % line_size, 0);
        assert!(config_data.len() == index_array_size + config_size);
        assert_eq!(new_config_lines_data.len(), num_lines * line_size);
        assert!(starting_line as usize + num_lines <= gumball_header.total_items_added);
        let (_, config_lines_data) = config_data.split_at_mut(index_array_size);
        config_lines_data[starting_line as usize * line_size..]
            .iter_mut()
            .take(num_lines)
            .enumerate()
            .for_each(|(i, l)| *l = new_config_lines_data[i]);
        Ok(())
    }

    pub fn update_config_metadata(
        ctx: Context<UpdateConfigMetadata>,
        url_base: Option<[u8; 64]>,
        name_base: Option<[u8; 32]>,
        symbol: Option<[u8; 32]>,
        seller_fee_basis_points: Option<u16>,
        is_mutable: Option<bool>,
        price: Option<u64>,
        retain_authority: Option<bool>,
        go_live_date: Option<i64>,
        authority: Option<Pubkey>,
        bot_wallet: Option<Pubkey>,
        max_mint_size: Option<u64>,
    ) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, _) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let mut gumball_machine = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        assert_eq!(gumball_machine.authority, ctx.accounts.authority.key());
        match url_base {
            Some(ub) => gumball_machine.url_base = ub,
            None => {}
        }
        match name_base {
            Some(nb) => gumball_machine.name_base = nb,
            None => {}
        }
        match symbol {
            Some(s) => gumball_machine.symbol = s,
            None => {}
        }
        match seller_fee_basis_points {
            Some(s) => gumball_machine.seller_fee_basis_points = s,
            None => {}
        }
        match is_mutable {
            Some(im) => gumball_machine.is_mutable = im.into(),
            None => {}
        }
        match retain_authority {
            Some(ra) => gumball_machine.retain_authority = ra.into(),
            None => {}
        }
        match price {
            Some(p) => gumball_machine.price = p,
            None => {}
        }
        match go_live_date {
            Some(gld) => gumball_machine.go_live_date = gld,
            None => {}
        }
        match authority {
            Some(a) => gumball_machine.authority = a,
            None => {}
        }
        match bot_wallet {
            Some(bw) => gumball_machine.bot_wallet = bw,
            None => {}
        }
        match max_mint_size {
            Some(mms) => gumball_machine.max_mint_size = mms.max(1).min(gumball_machine.max_items),
            None => {}
        }
        Ok(())
    }

    pub fn dispense(ctx: Context<Dispense>, num_items: u64) -> Result<()> {
        // Load all data
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, config_data) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        let clock = Clock::get()?;
        assert!(clock.unix_timestamp > gumball_header.go_live_date);
        let size = gumball_header.max_items as usize;
        let index_array_size = std::mem::size_of::<u32>() * size;
        let config_size = gumball_header.extension_len * size;
        let line_size = gumball_header.extension_len;

        assert!(config_data.len() == index_array_size + config_size);
        let (indices_data, config_lines_data) = config_data.split_at_mut(index_array_size);

        // TODO: Validate data

        let mut indices = cast_slice_mut::<u8, u32>(indices_data);
        for _ in 0..(num_items as usize).max(1).min(gumball_header.remaining) {
            // Get 8 bytes of entropy from the SlotHashes sysvar
            let mut buf: [u8; 8] = [0; 8];
            buf.copy_from_slice(
                &hashv(&[
                    &ctx.accounts.recent_blockhashes.data.borrow(),
                    &gumball_header.remaining.to_le_bytes(),
                ])
                .as_ref()[..8],
            );
            let entropy = u64::from_le_bytes(buf);
            // Shuffle the list of indices using Fisher-Yates
            let selected = entropy % gumball_header.remaining as u64;
            gumball_header.remaining -= 1;
            (&mut indices).swap(selected as usize, gumball_header.remaining);
            // Pull out config line from the data
            let random_config_index = indices[gumball_header.remaining] as usize * line_size;
            let config_line =
                config_lines_data[random_config_index..random_config_index + line_size].to_vec();

            let message = get_metadata_args(
                gumball_header.url_base,
                gumball_header.name_base,
                gumball_header.symbol,
                gumball_header.seller_fee_basis_points,
                gumball_header.is_mutable != 0,
                gumball_header.collection_key,
                None,
                gumball_header.creator_address,
                random_config_index,
                config_line,
            );

            let seed = ctx.accounts.gumball_machine.key();
            let seeds = &[seed.as_ref(), &[*ctx.bumps.get("willy_wonka").unwrap()]];
            let authority_pda_signer = &[&seeds[..]];
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.bubblegum.to_account_info(),
                bubblegum::cpi::accounts::Mint {
                    mint_authority: ctx.accounts.willy_wonka.to_account_info(),
                    authority: ctx.accounts.bubblegum_authority.to_account_info(),
                    nonce: ctx.accounts.nonce.to_account_info(),
                    gummyroll_program: ctx.accounts.gummyroll.to_account_info(),
                    owner: ctx.accounts.payer.to_account_info(),
                    delegate: ctx.accounts.payer.to_account_info(),
                    merkle_slab: ctx.accounts.merkle_slab.to_account_info(),
                },
                authority_pda_signer,
            );
            bubblegum::cpi::mint(cpi_ctx, message)?;
        }
        Ok(())
    }

    /// Reclaim gumball_machine lamports to authority
    pub fn destroy(ctx: Context<Destroy>) -> Result<()> {
        let mut gumball_machine_data = ctx.accounts.gumball_machine.try_borrow_mut_data()?;
        let (mut header_bytes, _) =
            gumball_machine_data.split_at_mut(std::mem::size_of::<GumballMachineHeader>());
        let gumball_header = GumballMachineHeader::load_mut_bytes(&mut header_bytes)?;
        assert!(gumball_header.authority == ctx.accounts.authority.key());
        let dest_starting_lamports = ctx.accounts.authority.lamports();
        **ctx.accounts.authority.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(ctx.accounts.gumball_machine.lamports())
            .ok_or(ProgramError::InvalidAccountData)?;
        **ctx.accounts.gumball_machine.lamports.borrow_mut() = 0;
        Ok(())
    }
}