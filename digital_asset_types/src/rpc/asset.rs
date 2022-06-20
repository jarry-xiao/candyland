use {
    serde::{Serialize, Deserialize},
    std::collections::HashMap,
};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct AssetProof {
    pub proof: Vec<String>,
    pub node_index: u64,
    pub tree_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Interface {
    #[serde(rename = "NFT1.0")]
    NftOneZero,
    #[serde(rename = "NFT")]
    Nft,
    #[serde(rename = "FungibleAsset")]
    FungibleAsset,
    #[serde(rename = "Custom")]
    Custom,
    #[serde(rename = "Identity")]
    Identity,
    #[serde(rename = "Executable")]
    Executable
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Quality {
    #[serde(rename = "$$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Context {
    #[serde(rename = "wallet-default")]
    WalletDefault,
    #[serde(rename = "web-desktop")]
    WebDesktop,
    #[serde(rename = "web-mobile")]
    WebMobile,
    #[serde(rename = "app-mobile")]
    AppMobile,
    #[serde(rename = "app-desktop")]
    AppDesktop,
    #[serde(rename = "app")]
    App,
    #[serde(rename = "vr")]
    Vr,
}

pub type Contexts = Vec<Context>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct File {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Contexts>,
}

pub type Files = Vec<File>;
pub type MetadataItem = HashMap<String, serde_json::Value>;
pub type Metadata = Vec<MetadataItem>;
pub type ObjectHAgrRKSz = HashMap<String, serde_json::Value>;
pub type Links = Vec<ObjectHAgrRKSz>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Content {
    #[serde(rename = "$schema")]
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Files>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
}

pub type Authority = HashMap<String, serde_json::Value>;
pub type Authorities = Vec<Authority>;
pub type Eligible = bool;
pub type Compressed = bool;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Compression {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligible: Option<Eligible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed: Option<Compressed>,
}

pub type GroupKey = String;
pub type GroupValue = String;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Group {
    #[serde(rename = "$$schema")]
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum RoyaltyModel {
    #[serde(rename = "creators")]
    Creators,
    #[serde(rename = "fanout")]
    Fanout,
    #[serde(rename = "single")]
    Single,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Royalty {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub royalty_model: Option<RoyaltyModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
}

pub type Address = String;
pub type Share = String;
pub type Verified = bool;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Creator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum OwnershipModel {
    #[serde(rename = "Single")]
    Single,
    #[serde(rename = "Token")]
    Token,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Ownership {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frozen: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ownership_model: Option<OwnershipModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Asset {
    pub interface: Interface,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorities: Option<Authorities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<Compression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping: Option<Vec<Group>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub royalty: Option<Royalty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creators: Option<Vec<Creator>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ownership: Option<Ownership>,
}