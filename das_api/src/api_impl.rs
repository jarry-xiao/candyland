use {
    crate::api::ApiContract,
    crate::config::Config,
    crate::validation::validate_pubkey,
    crate::DasApiError,
    async_trait::async_trait,
    digital_asset_types::{
        dapi::change_logs::*,
        rpc::{
            filter::{AssetSorting, OfferSorting},
            response::{AssetList, ListingsList, OfferList},
            AssetProof,
        },
    },
    sea_orm::{DatabaseConnection, DbErr, SqlxPostgresConnector},
    sqlx::postgres::PgPoolOptions,
};

pub struct DasApi {
    db_connection: DatabaseConnection,
}

impl DasApi {
    pub async fn from_config(config: Config) -> Result<Self, DasApiError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&*config.database_url)
            .await?;

        let conn = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
        Ok(DasApi {
            db_connection: conn,
        })
    }
}

pub fn not_found(asset_id: &String) -> DbErr {
    DbErr::RecordNotFound(format!("Asset Proof for {} Not Found", asset_id))
}

#[async_trait]
impl ApiContract for DasApi {
    async fn get_asset_proof(self: &DasApi, asset_id: String) -> Result<AssetProof, DasApiError> {
        let id = validate_pubkey(asset_id.clone())?;
        let id_bytes = id.to_bytes().to_vec();
        get_proof_for_asset(&self.db_connection, id_bytes)
            .await
            .and_then(|list| {
                if list.len() == 0 {
                    return Err(not_found(&asset_id));
                }
                Ok(list)
            })
            .and_then(|list| {
                let leaf = list.first().ok_or(not_found(&asset_id))?;
                Ok(AssetProof {
                    proof: list
                        .iter()
                        .map(|model| bs58::encode(&model.hash).into_string())
                        .collect(),
                    node_index: leaf.node_idx,
                    tree_id: bs58::encode(&leaf.tree).into_string(),
                })
            })
            .map_err(Into::into)
    }

    async fn get_assets_by_owner(
        &mut self,
        owner_address: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<AssetList, DasApiError> {
        todo!()
    }

    async fn get_listed_assets_by_owner(
        &mut self,
        owner_address: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<ListingsList, DasApiError> {
        todo!()
    }

    async fn get_offers_by_owner(
        &mut self,
        owner_address: String,
        sort_by: OfferSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<OfferList, DasApiError> {
        todo!()
    }

    async fn get_assets_by_group(
        &mut self,
        group_expression: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<AssetList, DasApiError> {
        todo!()
    }

    async fn get_assets_by_creator(
        &mut self,
        creator_expression: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<AssetList, DasApiError> {
        todo!()
    }

    async fn search_assets(
        &mut self,
        search_expression: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<AssetList, DasApiError> {
        todo!()
    }
}
