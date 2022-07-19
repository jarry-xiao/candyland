use std::io;

use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program, sysvar,
};
use anchor_lang::*;
use mpl_auction_house::{
    pda::{
        find_auction_house_address, find_auction_house_fee_account_address,
        find_auction_house_treasury_address, find_auctioneer_pda,
        find_auctioneer_trade_state_address, find_bid_receipt_address, find_escrow_payment_address,
        find_listing_receipt_address, find_program_as_signer_address,
        find_public_bid_trade_state_address, find_purchase_receipt_address,
        find_trade_state_address,
    },
    AuctionHouse, AuthorityScope,
};

use mpl_testing_utils::{solana::airdrop, utils::Metadata};
use std::result::Result as StdResult;

use bubblegum;
use solana_program_test::*;
use solana_sdk::{instruction::Instruction, transaction::Transaction, transport::TransportError};
use spl_associated_token_account::get_associated_token_address;

pub fn bubble_gum_program_test() -> ProgramTest {
    let mut program = ProgramTest::new("bubblegum", bubblegum::id(), None);
    program.add_program("bubblegum", bubblegum::id(), None);
    program
}

pub async fn create_and_insert_asset(
    context: &mut ProgramTestContext,

) -> StdResult<Pubkey, TransportError> {
    let accounts = bubblegum::accounts::MintV1 {
  
    }
    .to_account_metas(None);

    let data = bubblegum::instruction::MintV1 {
       
    }
    .data();

    let instruction = Instruction {
        program_id: bubblegum::id(),
        data,
        accounts,
    };

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_wallet.pubkey()),
        &[payer_wallet],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await?
}
