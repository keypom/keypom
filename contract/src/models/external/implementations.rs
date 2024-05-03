use crate::*;

impl ExtAsset {
    /// Convert an `ExtAsset` into an `InternalAsset`
    pub fn to_internal_asset(&self) -> InternalAsset {
        match self {
            ExtAsset::FTAsset(ft_data) => InternalAsset::ft(InternalFTData::new(
                ft_data.ft_contract_id.clone(),
                ft_data.registration_cost.into(),
            )),
            ExtAsset::NFTAsset(nft_data) => {
                InternalAsset::nft(InternalNFTData::new(nft_data.nft_contract_id.clone()))
            }
            ExtAsset::FCAsset(method_data) => InternalAsset::fc(FCData::new(method_data.clone())),
            ExtAsset::NearAsset(_) => InternalAsset::near,
        }
    }

    /// Standard function to check how many tokens a given asset transfers per use
    pub fn get_tokens_per_use(&self) -> U128 {
        match self {
            ExtAsset::FTAsset(ft_data) => ft_data.ft_amount,
            ExtAsset::NFTAsset(_) => U128(0),
            ExtAsset::NearAsset(near_data) => near_data.yoctonear,
            ExtAsset::FCAsset(_) => U128(0),
        }
    }

    /// Standard function to query for the asset ID of the current external asset
    pub fn get_asset_id(&self) -> AssetId {
        match self {
            ExtAsset::FTAsset(ft_data) => ft_data.ft_contract_id.to_string(),
            ExtAsset::NFTAsset(nft_data) => nft_data.nft_contract_id.to_string(),
            ExtAsset::NearAsset(_) => NEAR_ASSET_ID.to_string(),
            ExtAsset::FCAsset(_) => {
                near_sdk::log!("FC Asset does not have an asset ID. THIS SHOULD NEVER HAPPEN");
                FALLBACK_FC_ASSET_ID.to_string()
            }
        }
    }
}
