use {
    chrono::Utc,
    lazy_static::lazy_static,
    crate::{
        events::handle_event,
        parsers::{InstructionBundle, ProgramHandler, ProgramHandlerConfig},
        utils::{filter_events_from_logs},
        error::IngesterError,
    },
    num_traits::FromPrimitive,
    sea_orm::{
        entity::*,
        query::*,
        DatabaseConnection,
        JsonValue,
        SqlxPostgresConnector,
        TransactionTrait,
        DatabaseTransaction,
    },
    solana_sdk::pubkeys,
    digital_asset_types::{
        json::ChainDataV1,
        dao::{
            asset,
            asset_data,
            asset_creators,
            asset_authority,
            asset_grouping,
            sea_orm_active_enums::{ChainMutability, Mutability, OwnerType, RoyaltyTargetType},
        },
    },

    anchor_client::anchor_lang::{
        self,
        AnchorDeserialize,
        prelude::Pubkey,
    },
    bubblegum::state::leaf_schema::LeafSchemaEvent,
    flatbuffers::{ForwardsUOffset, Vector},
    plerkle_serialization::transaction_info_generated::transaction_info::{self},
    solana_sdk,
    sqlx::{self, Pool, Postgres},
    async_trait::async_trait,
};

use bubblegum::state::leaf_schema::{LeafSchema, Version};
use serde_json;
use bubblegum::state::NFTDecompressionEvent;
use digital_asset_types::adapter::{TokenStandard, UseMethod, Uses};
use crate::{get_gummy_roll_events, save_changelog_events};
use crate::utils::{bytes_from_fb_table};

pubkeys!(
    BubblegumProgramID,
    "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY"
);

pub struct BubblegumHandler {
    id: Pubkey,
    storage: DatabaseConnection,
}

#[async_trait]
impl ProgramHandler for BubblegumHandler {
    fn id(&self) -> Pubkey {
        self.id
    }

    fn config(&self) -> &ProgramHandlerConfig {
        lazy_static! {
            static ref CONFIG: ProgramHandlerConfig = ProgramHandlerConfig {
                responds_to_instruction: true,
                responds_to_account: false
            };
        }
        return &CONFIG;
    }

    async fn handle_instruction(&self, bundle: &InstructionBundle) -> Result<(), IngesterError> {
        handle_bubblegum_instruction(
            &bundle.instruction,
            &bundle.instruction_logs,
            &bundle.keys,
            &self.storage,
        )
            .await
    }
}

impl BubblegumHandler {
    pub fn new(pool: Pool<Postgres>) -> Self {
        BubblegumHandler {
            id: BubblegumProgramID(),
            storage: SqlxPostgresConnector::from_sqlx_postgres_pool(pool),
        }
    }
}

fn get_leaf_event(logs: &Vec<&str>) -> Result<LeafSchemaEvent, IngesterError>  {
    get_bubblegum_event(logs)
}

fn get_decompress_event(logs: &Vec<&str>) -> Result<NFTDecompressionEvent, IngesterError>  {
    get_bubblegum_event(logs)
}

fn get_bubblegum_event<T: anchor_lang::Event + anchor_lang::AnchorDeserialize>(logs: &Vec<&str>) -> Result<T, IngesterError> {
    let event_logs = filter_events_from_logs(logs);
    if event_logs.is_err() {
        println!("Error finding event logs in bubblegum logs");
        return Err(IngesterError::CompressedAssetEventMalformed);
    }

    let mut found_event: Option<T> = None;
    for event in event_logs.unwrap() {
        let event = handle_event::<T>(event);
        if event.is_ok() {
            found_event = event.ok()
        }
    }
    found_event.ok_or(IngesterError::CompressedAssetEventMalformed)
}


async fn tree_change_only<'a>(db: &DatabaseConnection, logs: &Vec<&'a str>) -> Result<(), IngesterError> {
    let gummy_roll_events = get_gummy_roll_events(logs)?;
    db.transaction::<_, _, IngesterError>(|txn| {
        Box::pin(async move {
            save_changelog_events(gummy_roll_events, txn).await?;
            Ok(())
        })
    })
        .await
        .map_err(Into::into)
}

async fn update_asset(txn: &DatabaseTransaction, id: Vec<u8>, model: asset::ActiveModel) -> Result<asset::Model, IngesterError> {
    asset::Entity::update(model)
        .filter(asset::Column::Id.eq(id))
        .exec(txn)
        .await
        .map_err(Into::into)
}

async fn handle_bubblegum_instruction<'a, 'b>(
    instruction: &'a transaction_info::CompiledInstruction<'a>,
    logs: &Vec<&'a str>,
    keys: &Vector<'b, ForwardsUOffset<transaction_info::Pubkey<'b>>>,
    db: &DatabaseConnection,
) -> Result<(), IngesterError> {
    let ix_type = bubblegum::get_instruction_type(instruction.data().unwrap());
    match ix_type {
        bubblegum::InstructionName::Transfer => {
            println!("BGUM: Transfer");
            let gummy_roll_events = get_gummy_roll_events(logs)?;
            let leaf_event = get_leaf_event(logs)?;
            db.transaction::<_, _, IngesterError>(|txn| {
                Box::pin(async move {
                    save_changelog_events(gummy_roll_events, txn).await?;
                    match leaf_event.schema {
                        LeafSchema::V1 {
                            nonce: _,
                            id,
                            owner,
                            ..
                        } => {
                            let owner_bytes = owner.to_bytes().to_vec();
                            let id_bytes = id.to_bytes().to_vec();
                            let asset_to_update = asset::ActiveModel {
                                id: Unchanged(id_bytes.clone()),
                                leaf: Set(Some(leaf_event.schema.to_node().to_vec())),
                                delegate: Set(None),
                                owner: Set(owner_bytes),
                                ..Default::default()
                            };
                            update_asset(txn, id_bytes, asset_to_update).await
                        }
                        _ => Err(IngesterError::NotImplemented)
                    }
                })
            }).await?;
        }
        bubblegum::InstructionName::Burn => {
            println!("BGUM: Burn");
            let gummy_roll_events = get_gummy_roll_events(logs)?;
            let leaf_event = get_leaf_event(logs)?;
            db.transaction::<_, _, IngesterError>(|txn| {
                Box::pin(async move {
                    save_changelog_events(gummy_roll_events, txn).await?;
                    match leaf_event.schema {
                        LeafSchema::V1 {
                            id,
                            delegate,
                            ..
                        } => {
                            let _delegate_bytes = delegate.to_bytes().to_vec();
                            let id_bytes = id.to_bytes().to_vec();
                            let asset_to_update = asset::ActiveModel {
                                id: Unchanged(id_bytes.clone()),
                                burnt_at: Set(Some(Utc::now())),
                                ..Default::default()
                            };
                            update_asset(txn, id_bytes, asset_to_update).await
                        }
                        _ => Err(IngesterError::NotImplemented)
                    }
                })
            }).await?;
        }
        bubblegum::InstructionName::Delegate => {
            println!("BGUM: Delegate");
            let gummy_roll_events = get_gummy_roll_events(logs)?;
            let leaf_event = get_leaf_event(logs)?;
            db.transaction::<_, _, IngesterError>(|txn| {
                Box::pin(async move {
                    save_changelog_events(gummy_roll_events, txn).await?;
                    match leaf_event.schema {
                        LeafSchema::V1 {
                            id,
                            delegate,
                            ..
                        } => {
                            let delegate_bytes = delegate.to_bytes().to_vec();
                            let id_bytes = id.to_bytes().to_vec();
                            let asset_to_update = asset::ActiveModel {
                                id: Unchanged(id_bytes.clone()),
                                leaf: Set(Some(leaf_event.schema.to_node().to_vec())),
                                delegate: Set(Some(delegate_bytes)),
                                ..Default::default()
                            };
                            update_asset(txn, id_bytes, asset_to_update).await
                        }
                        _ => Err(IngesterError::NotImplemented)
                    }
                })
            }).await?;
        }
        bubblegum::InstructionName::MintV1 => {
            println!("BGUM: MINT");
            let gummy_roll_events = get_gummy_roll_events(logs)?;
            let leaf_event = get_leaf_event(logs)?;
            let data = instruction.data().unwrap()[8..].to_owned();
            let data_buf = &mut data.as_slice();
            let ix: bubblegum::instruction::MintV1 =
                bubblegum::instruction::MintV1::deserialize(data_buf).unwrap();
            let accounts = instruction.accounts().unwrap();
            let update_authority = bytes_from_fb_table(keys, accounts[0] as usize);
            let owner = bytes_from_fb_table(keys, accounts[3] as usize);
            let delegate = bytes_from_fb_table(keys, accounts[4] as usize);
            let merkle_slab = bytes_from_fb_table(keys, accounts[5] as usize);
            db.transaction::<_, _, IngesterError>(|txn| {
                Box::pin(async move {
                    save_changelog_events(gummy_roll_events, txn).await?;
                    match leaf_event.schema {
                        LeafSchema::V1 {
                            nonce,
                            id,
                            ..
                        } => {
                            let metadata = ix.message;
                            // Printing metadata instruction arguments for debugging
                            println!(
                                "\tMetadata info: {} {} {} {}",
                                &metadata.name,
                                metadata.seller_fee_basis_points,
                                metadata.primary_sale_happened,
                                metadata.is_mutable,
                            );
                            let chain_data = ChainDataV1 {
                                name: metadata.name,
                                symbol: metadata.symbol,
                                edition_nonce: metadata.edition_nonce,
                                primary_sale_happened: metadata.primary_sale_happened,
                                token_standard: metadata.token_standard.and_then(|ts| {
                                    TokenStandard::from_u8(ts as u8)
                                }),
                                uses: metadata.uses.map(|u| {
                                    Uses {
                                        use_method: UseMethod::from_u8(u.use_method as u8).unwrap(),
                                        remaining: u.remaining,
                                        total: u.total,
                                    }
                                }),
                            };
                            let chain_data_json = serde_json::to_value(chain_data).map_err(|e| {
                                IngesterError::DeserializationError(e.to_string())
                            })?;
                            let chain_mutability = match metadata.is_mutable {
                                true => ChainMutability::Mutable,
                                false => ChainMutability::Immutable
                            };

                            let data = asset_data::ActiveModel {
                                chain_data_mutability: Set(chain_mutability),
                                schema_version: Set(1),
                                chain_data: Set(chain_data_json),
                                metadata_url: Set(metadata.uri),
                                metadata: Set(JsonValue::String("processing".to_string())),
                                metadata_mutability: Set(Mutability::Mutable),
                                ..Default::default()
                            }.insert(txn).await
                                .map_err(|txn_err| {
                                    IngesterError::StorageWriteError(txn_err.to_string())
                                })?;
                            let delegate = if owner == delegate {
                                None
                            } else {
                                Some(delegate)
                            };
                            asset::ActiveModel {
                                id: Set(id.to_bytes().to_vec()),
                                owner: Set(owner),
                                owner_type: Set(OwnerType::Single),
                                delegate: Set(delegate),
                                frozen: Set(false),
                                supply: Set(1),
                                supply_mint: Set(None),
                                compressed: Set(true),
                                compressible: Set(false),
                                tree_id: Set(Some(merkle_slab)),
                                nonce: Set(nonce as i64),
                                leaf: Set(Some(leaf_event.schema.to_node().to_vec())),
                                /// Get gummy roll seq
                                royalty_target_type: Set(RoyaltyTargetType::Creators),
                                royalty_target: Set(None),
                                royalty_amount: Set(metadata.seller_fee_basis_points as i32), //basis points
                                chain_data_id: Set(Some(data.id)),
                                ..Default::default()
                            }.insert(txn)
                                .await
                                .map_err(|txn_err| {
                                    IngesterError::StorageWriteError(txn_err.to_string())
                                })?;
                            if metadata.creators.len() > 0 {
                                let mut creators = Vec::with_capacity(metadata.creators.len());
                                for c in metadata.creators {
                                    creators.push(asset_creators::ActiveModel {
                                        asset_id: Set(id.to_bytes().to_vec()),
                                        creator: Set(c.address.to_bytes().to_vec()),
                                        share: Set(c.share as i32),
                                        verified: Set(c.verified),
                                        ..Default::default()
                                    });
                                }
                                asset_creators::Entity::insert_many(creators)
                                    .exec(txn)
                                    .await
                                    .map_err(|txn_err| {
                                        IngesterError::StorageWriteError(txn_err.to_string())
                                    })?;
                            }
                            asset_authority::ActiveModel {
                                asset_id: Set(id.to_bytes().to_vec()),
                                authority: Set(update_authority),
                                ..Default::default()
                            }.insert(txn)
                                .await
                                .map_err(|txn_err| {
                                    IngesterError::StorageWriteError(txn_err.to_string())
                                })?;
                            if let Some(c) = metadata.collection {
                                if c.verified {
                                    asset_grouping::ActiveModel {
                                        asset_id: Set(id.to_bytes().to_vec()),
                                        group_key: Set("collection".to_string()),
                                        group_value: Set(c.key.to_string()),
                                        ..Default::default()
                                    }.insert(txn)
                                        .await
                                        .map_err(|txn_err| {
                                            IngesterError::StorageWriteError(txn_err.to_string())
                                        })?;
                                }
                            }
                            Ok(())
                        }
                        _ => {
                            Err(IngesterError::NotImplemented)
                        }
                    }
                })
            }).await?;
        }
        bubblegum::InstructionName::Redeem => {
            println!("Bubblegum: Redeem");
            tree_change_only(db, logs).await?;
        }
        bubblegum::InstructionName::CancelRedeem => {
            println!("Bubblegum: Cancel Redeem");
            tree_change_only(db, logs).await?;
        }
        bubblegum::InstructionName::DecompressV1 => {
            println!("BGUM: Decompress");
            let decompress_event = get_decompress_event(logs)?;
            db.transaction::<_, _, IngesterError>(|txn| {
                Box::pin(async move {
                    match decompress_event.version {
                        Version::V1 => {
                            let id_bytes = decompress_event.id.to_bytes().to_vec();
                            let asset_to_update = asset::ActiveModel {
                                id: Unchanged(id_bytes.clone()),
                                leaf: Set(None),
                                compressed: Set(false),
                                compressible: Set(false),
                                supply: Set(1),
                                supply_mint: Set(Some(id_bytes.clone())),
                                ..Default::default()
                            };
                            update_asset(txn, id_bytes, asset_to_update).await
                        }
                        _ => Err(IngesterError::NotImplemented)
                    }
                })
            }).await?;
        }
        _ => println!("Bubblegum: Not Implemented Instruction"),
    }
    Ok(())
}
