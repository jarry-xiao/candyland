use crate::{DasApiError, RpcModule};
use async_trait::async_trait;
use digital_asset_types::rpc::filter::{AssetSorting, OfferSorting};
use digital_asset_types::rpc::response::{AssetList, ListingsList, OfferList};
use digital_asset_types::rpc::{Asset, AssetProof};

#[async_trait]
pub trait ApiContract: Send + Sync + 'static {
    async fn check_health(&self) -> Result<(), DasApiError>;
    async fn get_asset_proof(&self, asset_id: String) -> Result<AssetProof, DasApiError>;
    async fn get_asset(&self, asset_id: String) -> Result<Asset, DasApiError>;
    async fn get_assets_by_owner(
        &mut self,
        owner_address: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<AssetList, DasApiError>;
    async fn get_listed_assets_by_owner(
        &mut self,
        owner_address: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<ListingsList, DasApiError>;
    async fn get_offers_by_owner(
        &mut self,
        owner_address: String,
        sort_by: OfferSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<OfferList, DasApiError>;
    async fn get_assets_by_group(
        &mut self,
        group_expression: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<AssetList, DasApiError>;
    async fn get_assets_by_creator(
        &mut self,
        creator_expression: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<AssetList, DasApiError>;
    async fn search_assets(
        &mut self,
        search_expression: String,
        sort_by: AssetSorting,
        limit: u32,
        page: u32,
        before: String,
        after: String,
    ) -> Result<AssetList, DasApiError>;
}

pub struct RpcApiBuilder;

impl<'a> RpcApiBuilder {
    pub fn build(
        contract: Box<dyn ApiContract>,
    ) -> Result<RpcModule<Box<dyn ApiContract>>, DasApiError> {
        let mut module = RpcModule::new(contract);

        module.register_async_method("healthz", |rpc_params, rpc_context| async move {
            println!("Checking Health");
            rpc_context.check_health()
                .await
                .map_err(Into::into)
        })?;

        module.register_async_method("get_asset_proof", |rpc_params, rpc_context| async move {
            let asset_id = rpc_params.one::<String>()?;
            println!("Asset Id {}", asset_id);
            rpc_context
                .get_asset_proof(asset_id)
                .await
                .map_err(Into::into)
        })?;
        module.register_async_method("get_asset", |rpc_params, rpc_context| async move {
            let asset_id = rpc_params.one::<String>()?;
            println!("Asset Id {}", asset_id);
            rpc_context
                .get_asset(asset_id)
                .await
                .map_err(Into::into)
        })?;

        Ok(module)
    }
}