use near_sdk::collections::{UnorderedMap, LazyOption};

use crate::*;

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKeys {
    AssetIdsByUse { drop_id_hash: CryptoHash },
    AssetById { drop_id_hash: CryptoHash },
    KeyInfoByPk { drop_id_hash: CryptoHash },
    DropMetadata { drop_id_hash: CryptoHash },
    DropById,
    DropIdByPk,
    UserBalances
}

/// Internal drop data that is stored in the contract
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalDrop {
    /// Account ID who funded / owns the rights to this specific drop
    pub funder_id: AccountId,
    /// Metadata for the drop in the form of stringified JSON. The format is completely up to the
    /// user and there are no standards for format.
    pub metadata: LazyOption<DropMetadata>,
    /// How many uses there are per key in the drop. This should be equal to the length of keys in assets_metadata_by_use
    pub uses_per_key: UseNumber,
    /// Map an asset ID to a specific asset. This is a hyper optimization so the asset data isn't repeated in the contract
    pub asset_by_id: UnorderedMap<AssetId, InternalAsset>,
    /// For every use number, keep track of what assets there are.
    pub key_behavior_by_use: LookupMap<UseNumber, KeyBehavior>,
    /// Set of public keys associated with this drop mapped to their specific key information.
    pub key_info_by_pk: UnorderedMap<PublicKey, InternalKeyInfo>,
    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64
}

impl InternalDrop {
    /// Convert an `InternalDrop` into an `ExtDrop`
    pub fn to_external_drop(&self) -> ExtDrop {
        let mut assets_per_use: HashMap<UseNumber, Vec<Option<ExtAsset>>> = HashMap::new();
        let internal_assets_data: Vec<InternalAsset> = self.asset_by_id.values().collect();
        
        // Loop through starting from 1 -> max_num_uses and add the assets to the hashmap
        for use_number in 1..=self.uses_per_key {
            let KeyBehavior {assets_metadata, config: _} = self.key_behavior_by_use.get(&use_number).expect("Use number not found");

            let mut assets: Vec<Option<ExtAsset>> = Vec::new();
            
            for metadata in assets_metadata {
                let asset = self.asset_by_id.get(&metadata.asset_id).unwrap();
                assets.push(asset.to_external_asset(&metadata));
            }
            assets_per_use.insert(use_number, assets);
        }

        ExtDrop {
            assets_per_use,
            internal_assets_data
        }
    }
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalKeyInfo {
    /// How many uses this key has left. Once 0 is reached, the key is deleted
    pub remaining_uses: UseNumber,

    /// Nonce for the current key.
    pub key_id: u64,
}

/// Outlines the different asset types that can be used in drops. This is the internal version of `ExtAsset`
/// And represents the data that is stored inside the Keypom contract to keep track of assets
#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum InternalAsset {
    ft(InternalFTData),
    none
}

impl InternalAsset {
    /// Convert an `InternalAsset` into an `ExtAsset`
    pub fn to_external_asset(self, asset_metadata: &AssetMetadata) -> Option<ExtAsset> {
        match self {
            InternalAsset::ft(ft_data) => Some(ExtAsset::FTAsset(ExtFTData {
                contract_id: ft_data.contract_id.clone(),
                registration_cost: ft_data.registration_cost.into(),
                // FTs should ALWAYS have a tokens_per_use value
                amount: asset_metadata.tokens_per_use.unwrap().into()
            })),
            InternalAsset::none => None
        }
    }

    /// Standard function for claiming an asset regardless of its type
    pub fn claim_asset(&mut self, drop_id: &DropId, receiver_id: &AccountId, tokens_per_use: &Option<Balance>) {
        match self {
            InternalAsset::ft(ref mut ft_data) => {
                ft_data.claim_ft_asset(drop_id, receiver_id, &tokens_per_use.unwrap())
            },
            InternalAsset::none => {}
        }
    }

    /// Standard function to check whether an asset is empty or not
    pub fn is_empty(&self) -> bool {
        match self {
            InternalAsset::ft(ft) => {
                if ft.balance_avail != 0 {
                    near_sdk::log!("There are {} FTs still in the drop. Please withdraw them before deleting.", ft.balance_avail);
                    return false;
                }
                return true;
            },
            InternalAsset::none => true
        }
    }

    /// Standard function for refunding assets
    /// This does not include any ext assets such as FTs or NFTs.
    /// This simply refunds the funder for the $NEAR cost associated with 1 key use for the given asset
    pub fn refund_amount(&self) -> Balance {
        match self {
            InternalAsset::ft(ft_data) => {
                return ft_data.registration_cost;
            },
            InternalAsset::none => 0
        }
    }

    /// Standard function to query how much gas it takes for 1 claim of a given asset
    pub fn get_required_gas(&self) -> Gas {
        match self {
            InternalAsset::ft(_) => GAS_FOR_CLAIM_LOGIC + MIN_GAS_FOR_FT_TRANSFER + MIN_GAS_FOR_STORAGE_DEPOSIT + MIN_GAS_FOR_RESOLVE_BATCH,
            InternalAsset::none => GAS_FOR_NONE_ASSET
        }
    }
}

/// Every use number has corresponding behavior data which includes information about all the assets in that use
#[derive(BorshDeserialize, BorshSerialize)]
pub struct KeyBehavior {
    /// Configurations for this specific use
    pub config: Option<bool>,
    /// Metadata for each asset in this use
    pub assets_metadata: Vec<AssetMetadata>
}

/// Metadata corresponding to a specific asset. This keeps track of the ID and optionally tokens being transferred per use
#[derive(BorshDeserialize, BorshSerialize)]
pub struct AssetMetadata {
    /// What asset is mapped to this specific use
    pub asset_id: AssetId,
    /// How many tokens should be transferred in this use?
    /// This is only relevant for FT and $NEAR assets
    pub tokens_per_use: Option<U128>,
}