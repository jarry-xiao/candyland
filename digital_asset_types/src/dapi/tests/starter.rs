#[cfg(test)]
mod tests {
    use sea_orm::{
        entity::prelude::*, entity::*, tests_cfg::*,
        DatabaseBackend, MockDatabase, MockExecResult, Transaction,
    };

    #[async_std::test]
    async fn test_get_asset() -> Result<(), DbErr> {
        // Create MockDatabase with mock execution result

 let merkle_slab = Keypair.generate().pubkey;
let db = MockDatabase::new(DatabaseBackend::Postgres);


        // is this needed ?
                    // save_changelog_events(gummy_roll_events, slot, txn).await?;
              let chain_data = ChainDataV1 {
                                name: "Honey Badges #1",
                                symbol: "HB",
                                edition_nonce: None,
                                primary_sale_happened: true,
                                token_standard: TokenStandard::NonFungible,
                                uses: None,
                            };


    let chain_data_json = serde_json::to_value(chain_data)?;

                            let chain_mutability =  ChainMutability::Mutable;

                            
              let data = asset_data::ActiveModel {
                                chain_data_mutability: Set(chain_mutability),
                                schema_version: Set(1),
                                chain_data: Set(chain_data_json),
                                metadata_url: Set(metadata.uri),
                                metadata: Set(JsonValue::String("processing".to_string())),
                                metadata_mutability: Set(Mutability::Mutable),
                                ..Default::default()
                            }.insert(txn).await?;

                         
        
          asset::ActiveModel {
                                id: Set(id.to_bytes().to_vec()),
                                owner: Set(owner),
                                owner_type: Set(OwnerType::Single),
                                delegate: Set(None),
                                frozen: Set(false),
                                supply: Set(1),
                                supply_mint: Set(None),
                                compressed: Set(true),
                                compressible: Set(false),
                                tree_id: Set(Some(merkle_slab)),
                                specification_version: Set(1),
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
                                ?;

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
                                    .await?;
                                 
                                    asset_authority::ActiveModel {
                                asset_id: Set(id.to_bytes().to_vec()),
                                authority: Set(update_authority),
                                ..Default::default()
                            }.insert(txn)
                                .await?;
                                

        // Insert the ActiveModel into MockDatabase
        assert_eq!(
            apple.clone().insert(&db).await?,
            cake::Model {
                id: 15,
                name: "Apple Pie".to_owned()
            }
        );

        // If you want to check the last insert id
        let insert_result = cake::Entity::insert(apple).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 16);

        // Checking transaction log
        assert_eq!(
            db.into_transaction_log(),
            vec![
                Transaction::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    r#"INSERT INTO "cake" ("name") VALUES ($1) RETURNING "id", "name""#,
                    vec!["Apple Pie".into()]
                ),
                Transaction::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    r#"INSERT INTO "cake" ("name") VALUES ($1) RETURNING "id""#,
                    vec!["Apple Pie".into()]
                ),
            ]
        );

        Ok(())
    }
}