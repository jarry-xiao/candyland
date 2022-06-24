use std::collections::HashMap;
use std::path::Path;
use mime_guess::Mime;
use sea_orm::{DatabaseConnection};
use sea_orm::{entity::*, query::*, DbErr};
use url::Url;
use crate::dao::{asset, asset_authority, asset_creators, asset_data, asset_grouping};
use crate::dao::prelude::{Asset, AssetData};
use crate::rpc::{Asset as RpcAsset, Authority, Compression, Content, File, Interface, Links, Scope};
use serde_json::Value;

pub fn to_uri(uri: String) -> Option<Url> {
    Url::parse(&*uri).ok()
}

pub fn get_mime(url: Url) -> Option<Mime> {
    mime_guess::from_path(Path::new(url.path())).first()
}

pub fn get_mime_type_from_uri(uri: String) -> Option<String> {
    to_uri(uri)
        .and_then(get_mime)
        .map(|m| { m.to_string() })
}

pub fn file_from_str(str: String) -> File {
    let mime = get_mime_type_from_uri(str.clone());
    File {
        uri: Some(str),
        mime,
        quality: None,
        contexts: None,
    }
}

pub fn track_top_level_file(file_map: &mut HashMap<String, File>, top_level_file: Option<&serde_json::Value>) {
    if top_level_file.is_some() {
        let img = top_level_file.and_then(|x|x.as_str()).unwrap();
        let entry = file_map.get(img);
        if entry.is_none() {
            file_map.insert(img.to_string(), file_from_str(img.to_string()));
        }
    }
}

fn v1_content_from_json(metadata: &serde_json::Value) -> Result<Content, DbErr> {
    // todo -> move this to the bg worker for pre processing
    let mut selector = jsonpath_lib::selector(metadata);
    println!("{}", metadata.to_string());
    let image = selector("$.image")
        .ok()
        .filter(|d|!Vec::is_empty(d))
        .as_mut()
        .and_then(|v| v.pop());
    let animation = selector("$.animation_url")
        .ok()
        .filter(|d|!Vec::is_empty(d))
        .as_mut()
        .and_then(|v| v.pop());
    let external_url = selector("$.external_url")
        .ok()
        .filter(|d|!Vec::is_empty(d))
        .map(|val| {
            let mut links = HashMap::new();
            links.insert("external_url".to_string(), val[0].to_owned());
            links
        });
    let mut actual_files: HashMap<String, File> = HashMap::new();
    selector("$.properties.files[*]").ok()
        .filter(|d|!Vec::is_empty(d))
        .map(|files| {
            for v in files.iter() {
                if v.is_object() {
                    let uri = v.get("uri");
                    let mime_type = v.get("type");
                    match (uri, mime_type) {
                        (Some(u), Some(m)) => {
                            let str_uri = u.as_str().unwrap().to_string();
                            let str_mime = m.as_str().unwrap().to_string();
                            actual_files.insert(str_uri.clone(), File {
                                uri: Some(str_uri),
                                mime: Some(str_mime),
                                quality: None,
                                contexts: None,
                            });
                        }
                        (Some(u), None) => {
                            let str_uri = serde_json::to_string(v).unwrap();
                            actual_files.insert(str_uri.clone(), file_from_str(str_uri));
                        }
                        _ => {}
                    }
                } else if v.is_string() {
                    let str_uri = v.as_str().unwrap().to_string();
                    actual_files.insert(str_uri.clone(), file_from_str(str_uri));
                }
            }
        });

    track_top_level_file(&mut actual_files, image);
    track_top_level_file(&mut actual_files, animation);
    let files: Vec<File> = actual_files.into_values().collect();
    Ok(Content {
        schema: "https://schema.metaplex.com/nft1.0.json".to_string(),
        files: Some(files),
        metadata: None,
        links: external_url,
    })
}

fn get_content(asset: &asset::Model, data: &asset_data::Model) -> Result<Content, DbErr> {
    match data.schema_version {
        1 => {
            v1_content_from_json(&data.metadata)
        }
        _ => Err(DbErr::Custom("Version Not Implemented".to_string()))
    }
}

pub fn to_authority(authority: Vec<asset_authority::Model>) -> Vec<Authority> {
    authority.iter().map(|a| {
        Authority {
            address: bs58::encode(&a.authority).into_string(),
            scopes: vec![Scope::Full]
        }
    }).collect()
}

pub async fn get_asset(db: &DatabaseConnection, asset_id: Vec<u8>) -> Result<RpcAsset, DbErr> {
    let asset_data: (asset::Model,
                     asset_data::Model) = Asset::find_by_id(asset_id)
        .find_also_related(AssetData)
        .one(db)
        .await
        .and_then(|o| {
            match o {
                Some((a, Some(d))) => Ok((a, d)),
                _ => Err(DbErr::RecordNotFound("Asset Not Found".to_string()))
            }
        })?;

    let (asset, data) = asset_data;

    let interface = match asset.specification_version {
        1 => Interface::NftOneZero,
        _ => Interface::Nft
    };

    let content = get_content(&asset, &data)?;
    let authorities: Vec<asset_authority::Model> = asset_authority::Entity::find()
        .filter(asset_authority::Column::AssetId.eq(asset.id.clone()))
        .all(db)
        .await?;
    let rpc_authorities = to_authority(authorities);


    Ok(RpcAsset {
        interface,
        id: bs58::encode(asset.id).into_string(),
        content: Some(content),
        authorities: Some(rpc_authorities),
        compression: Some(
            Compression{
                eligible: asset.compressible,
                compressed: asset.compressed
            }
        ),
        grouping: None,
        royalty: None,
        creators: None,
        ownership: None,
    })
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn simple_v1_content() {
        let doc = r#"
        {"name": "Handalf", "image": "https://arweave.net/UicDlez8No5ruKmQ1-Ik0x_NNxc40mT8NEGngWyXyMY", "attributes": [], "properties": {"files": ["https://arweave.net/UicDlez8No5ruKmQ1-Ik0x_NNxc40mT8NEGngWyXyMY"], "category": null}, "description": "The Second NFT ever minted from justmint.xyz", "external_url": ""}
        "#;

        let json: Value = serde_json::from_str(doc).unwrap();
        let mut selector = jsonpath_lib::selector(&json);
        let c: Content = v1_content_from_json(&json).unwrap();
        assert_eq!(c.files, Some(vec![File{
            uri: Some("https://arweave.net/UicDlez8No5ruKmQ1-Ik0x_NNxc40mT8NEGngWyXyMY".to_string()),
            mime: None,
            quality: None,
            contexts: None
        }]))
    }

}
