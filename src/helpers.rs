use crate::*;

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_drop_id(drop_id: &String) -> CryptoHash {
    env::sha256_array(drop_id.as_bytes())
}

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn asset_id_from_ext_asset(ext_asset: &ExtAsset) -> AssetId {
    match ext_asset {
        ExtAsset::FTAsset(ft_data) => ft_data.contract_id.to_string(),
        _ => env::panic_str("Asset type not supported")
    }
}