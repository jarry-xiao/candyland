use {
    crate::utils::{filter_events_from_logs, pubkey_from_fb_table},
    anchor_client::anchor_lang::AnchorDeserialize,
    bubblegum::state::leaf_schema::LeafSchemaEvent,
    flatbuffers::{ForwardsUOffset, Vector},
    nft_ingester::events::handle_event,
    plerkle_serialization::transaction_info_generated::transaction_info::{self},
    solana_sdk,
    sqlx::{self, types::Uuid, Pool, Postgres},
};

const SET_NFT_APPSQL: &str = r#"
    INSERT INTO nft_metadata (
        owner,      
        delegate,   
        nonce,      
        revision,   
        name,       
        symbol,     
        uri,
        sellerfeebasispoints,
        primarySaleHappened,
        isMutable
    )
    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) 
    "#;

fn get_bubblegum_leaf_event(logs: &Vec<String>) -> Result<LeafSchemaEvent, ()> {
    let event_logs = filter_events_from_logs(logs);
    if event_logs.is_err() {
        println!("Error finding event logs in bubblegum logs");
        return Err(());
    }

    let mut found_event: Option<LeafSchemaEvent> = None;
    for event in event_logs.unwrap() {
        match handle_event::<LeafSchemaEvent>(event) {
            Ok(leaf_event) => {
                if found_event.is_some() {
                    println!("\tOnly expected one leaf event per bubblegum ix");
                    return Err(());
                }
                found_event = Some(leaf_event);
            }
            Err(_) => {
                println!("\tMalformed bubblegum log event data");
                return Err(());
            }
        }
    }

    match found_event {
        Some(leaf_event) => Ok(leaf_event),
        _ => {
            println!("No bubblegum event found in logs");
            Err(())
        }
    }
}

pub async fn handle_bubblegum_instruction(
    instruction: &solana_sdk::instruction::CompiledInstruction,
    logs: &Vec<String>,
    keys: &Vector<'_, ForwardsUOffset<transaction_info::Pubkey<'_>>>,
    pid: i64,
    pool: &Pool<Postgres>,
) -> Result<(), ()> {
    match bubblegum::get_instruction_type(&instruction.data) {
        bubblegum::InstructionName::Transfer => {
            println!("Bubblegum: Transfer");
            // TODO(): insert uuid with new owner with a greater PID
        }
        bubblegum::InstructionName::Mint => {
            println!("Bubblegum: Mint");

            // Retrieve nonce value from the LeafSchemaEvent emitted
            let leaf_event_result = get_bubblegum_leaf_event(logs);
            if leaf_event_result.is_err() {
                println!("Could not find leaf event");
                return Err(());
            };
            let leaf_event = leaf_event_result.unwrap();

            let owner = pubkey_from_fb_table(keys, instruction.accounts[4] as usize);
            let delegate = pubkey_from_fb_table(keys, instruction.accounts[5] as usize);

            let data = instruction.data[8..].to_owned();
            let data_buf = &mut data.as_slice();
            let ix: bubblegum::instruction::Mint =
                bubblegum::instruction::Mint::deserialize(data_buf).unwrap();
            let metadata = ix.message;

            // Printing metadata instruction arguments for debugging
            println!(
                "\tMetadata info: {} {} {} {}",
                &metadata.name,
                metadata.seller_fee_basis_points,
                metadata.primary_sale_happened,
                metadata.is_mutable,
            );

            // TODO(): insert ALL metadata for NFT so that it can be hashed on-chain
            sqlx::query(SET_NFT_APPSQL)
                .bind(&bs58::decode(&owner).into_vec().unwrap())
                .bind(&bs58::decode(&delegate).into_vec().unwrap())
                .bind(&Uuid::from_u128(leaf_event.nonce))
                // revision
                .bind(&pid)
                .bind(&metadata.name)
                .bind(&metadata.symbol)
                .bind(&metadata.uri)
                .bind(metadata.seller_fee_basis_points as u32)
                .bind(metadata.primary_sale_happened)
                .bind(metadata.is_mutable)
                .execute(pool)
                .await
                .unwrap();
            println!("\tInserted metadata!");
        }
        // We should probably ignore Redeem & Cancel Redeem
        // Since redeemed voucher is non-transferable, the owner
        // actually technically still owns the metadata
        // i.e. even though gummyroll remove instruction executed in Redeem
        //      we should remove metadata only in the Decompress ix
        //      otherwise, it becomes hard to reinsert data on a CancelRedeem
        bubblegum::InstructionName::Redeem => {
            println!("Bubblegum: Redeem");
            // TODO(): nothing
        }
        bubblegum::InstructionName::CancelRedeem => {
            println!("Bubblegum: CancelRedeem");
            // TODO(): nothing
        }
        bubblegum::InstructionName::Decompress => {
            println!("Bubblegum: Decompress");
            // TODO(): set nonce uuid to a non-queryable state
        }
        _ => println!("Bubblegum: Ignored instruction"),
    }
    Ok(())
}
