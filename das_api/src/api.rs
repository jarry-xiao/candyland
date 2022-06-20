use {
    async_trait::async_trait
};
use digital_asset_types::rpc::AssetProof;
use digital_asset_types::rpc::filter::{AssetSorting, OfferSorting};
use digital_asset_types::rpc::response::{AssetList, AssetListings, ListingsList, OfferList};

#[async_trait]
pub trait ApiContract {
    async fn get_asset_proof(&mut self, asset_id: AssetId) -> RpcRequest<AssetProof>;
    async fn get_assets_by_owner(&mut self, owner_address: String, sort_by: AssetSorting, limit: u32, page: u32, before: String, after: String) -> RpcRequest<AssetList>;
    async fn get_listed_assets_by_owner(&mut self, owner_address: String, sort_by: AssetSorting, limit: u32, page: u32, before: String, after: String) -> RpcRequest<ListingsList>;
    async fn get_offers_by_owner(&mut self, owner_address: String, sort_by: OfferSorting, limit: u32, page: u32, before: String, after: String) -> RpcRequest<OfferList>;
    async fn get_assets_by_group(&mut self, group_expression: String, sort_by: AssetSorting, limit: u32, page: u32, before: String, after: String) -> RpcRequest<AssetList>;
    async fn get_assets_by_creator(&mut self, creator_expression: String, sort_by: AssetSorting, limit: u32, page: u32, before: String, after: String) -> RpcRequest<AssetList>;
    async fn search_assets(&mut self, search_expression: String, sort_by: AssetSorting, limit: u32, page: u32, before: String, after: String) -> RpcRequest<AssetList>;
}
