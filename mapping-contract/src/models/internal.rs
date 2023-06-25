use near_sdk::collections::UnorderedMap;

use crate::*;

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKeys {
    AssetIdsByUse { drop_id_hash: CryptoHash },
    AssetById { drop_id_hash: CryptoHash },
    DropById
}

/// Internal drop data that is stored in the contract
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalDrop {
    /// How many uses there are per key in the drop. This should be equal to the length of keys in assets_metadata_by_use
    pub uses_per_key: UseNumber,
    /// Map an asset ID to a specific asset. This is a hyper optimization so the asset data isn't repeated in the contract
    pub asset_by_id: UnorderedMap<AssetId, InternalAsset>,
    /// For every use number, keep track of what assets there are.
    pub assets_metadata_by_use: LookupMap<UseNumber, Vec<AssetMetadata>>,
}

/// Outlines the different asset types that can be used in drops. This is the internal version of `ExtAsset`
/// And represents the data that is stored inside the Keypom contract to keep track of assets
#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize)]
pub enum InternalAsset {
    ft(InternalFTData),
}

impl InternalAsset {
    /// Standard function for claiming an asset regardless of its type
    pub fn claim_asset(&mut self, drop_id: &DropId, receiver_id: &AccountId, tokens_per_use: &Option<u128>) {
        match self {
            InternalAsset::ft(ref mut ft_data) => {
                ft_data.claim_ft_asset(drop_id, receiver_id, &tokens_per_use.unwrap())
            },
            _ => env::panic_str("Asset type not supported")
        }
    }
}

/// Metadata corresponding to a specific asset. This keeps track of the ID and optionally tokens being transferred per use
#[derive(BorshDeserialize, BorshSerialize)]
pub struct AssetMetadata {
    pub asset_id: AssetId,
    pub tokens_per_use: Option<U128>,
}