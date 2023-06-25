use near_sdk::collections::UnorderedMap;

use crate::*;

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKeys {
    AssetIdsByUse { drop_id_hash: CryptoHash },
    AssetById { drop_id_hash: CryptoHash },
    DropById
}

/// Which specific use is something being acted on. This is not zero indexed (i.e the first use is 1)
pub type UseNumber = u16;
/// The ID of a given asset such as FTs, NFTs, or Function Calls.
pub type AssetId = String;
/// The ID for a given drop (this is the unique identifier for the drop and is how it will be referenced)
pub type DropId = String;

/// Outlines the different asset types that can be used in drops. This is the external version of `InternalAsset`
/// And represents the data that is passed into and out of the Keypom contract
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum ExtAsset {
    FTAsset(ExtFTData),
}

impl ExtAsset {
    /// Convert an `InternalAsset` into an `ExtAsset`
    pub fn from_internal_asset(internal_asset: &InternalAsset, asset_metadata: &AssetMetadata) -> Self {
        match internal_asset {
            InternalAsset::ft(ft_data) => ExtAsset::FTAsset(ExtFTData {
                contract_id: ft_data.contract_id.clone(),
                registration_cost: ft_data.registration_cost.into(),
                // FTs should ALWAYS have a tokens_per_use value
                tokens_per_use: asset_metadata.tokens_per_use.unwrap().into()
            })
        }
    }

    pub fn to_internal_asset(&self) -> InternalAsset {
        match self {
            ExtAsset::FTAsset(ft_data) => InternalAsset::ft(InternalFTData::new(
                ft_data.contract_id.clone(),
                ft_data.registration_cost.into()
            )),
            _ => env::panic_str("Asset type not supported")
        }
    }

    pub fn get_tokens_per_use(&self) -> U128 {
        match self {
            ExtAsset::FTAsset(ft_data) => ft_data.tokens_per_use.into(),
            _ => env::panic_str("Asset type not supported")
        }
    }

    pub fn get_cost_per_key(&self) -> u128 {
        match self {
            ExtAsset::FTAsset(ft_data) => ft_data.registration_cost.into(),
            _ => env::panic_str("Asset type not supported")
        }
    }
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtFTData {
    /// The contract that the fungible tokens originate from
    pub contract_id: AccountId,
    /// How much $NEAR (in yocto) it costs to register a new user on the fungible token contract
    pub registration_cost: U128,
    /// How many fungible tokens (in their smallest indivisible unit) should be transferred when the drop is claimed
    pub tokens_per_use: U128
}

/// Drop data being returned from view calls from Keypom
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtDrop {
    assets_by_use: HashMap<UseNumber, Vec<ExtAsset>>,
}

impl ExtDrop {
    /// Convert an `InternalDrop` into an `ExtDrop`
    pub fn from_internal_drop(internal_drop: &InternalDrop) -> Self {
        let mut assets_by_use: HashMap<UseNumber, Vec<ExtAsset>> = HashMap::new();
        
        // Loop through starting from 1 -> max_num_uses and add the assets to the hashmap
        for use_number in 1..=internal_drop.uses_per_key {
            let assets_metadata = internal_drop.assets_metadata_by_use.get(&use_number).unwrap();

            let mut assets: Vec<ExtAsset> = Vec::new();
            
            for metadata in assets_metadata {
                let asset = internal_drop.asset_by_id.get(&metadata.asset_id).unwrap();
                assets.push(ExtAsset::from_internal_asset(&asset, &metadata));
            }
            assets_by_use.insert(use_number, assets);
        }

        ExtDrop {
            assets_by_use
        }
    }
}

/// Outlines the different asset types that can be used in drops. This is the internal version of `ExtAsset`
/// And represents the data that is stored inside the Keypom contract to keep track of assets
#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize)]
pub enum InternalAsset {
    ft(InternalFTData),
}

impl InternalAsset {
    pub fn claim_asset(&mut self, drop_id: &DropId, receiver_id: &AccountId, tokens_per_use: &Option<u128>) {
        match self {
            InternalAsset::ft(ref mut ft_data) => {
                ft_data.claim_ft_asset(drop_id, receiver_id, &tokens_per_use.unwrap())
            },
            _ => env::panic_str("Asset type not supported")
        }
    }

    pub fn fund_asset(&mut self, drop_id: &DropId, receiver_id: &AccountId, tokens_per_use: &Option<u128>) {
        match self {
            InternalAsset::ft(ref mut ft_data) => {
                ft_data.claim_ft_asset(drop_id, receiver_id, &tokens_per_use.unwrap())
            },
            _ => env::panic_str("Asset type not supported")
        }
    }
}

/// Keep track of specific data related to an access key. This allows us to optionally refund funders later.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct AssetMetadata {
    pub asset_id: AssetId,
    pub tokens_per_use: Option<U128>,
}

/// Keep track of specific data related to an access key. This allows us to optionally refund funders later.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalDrop {
    pub uses_per_key: UseNumber,
    pub asset_by_id: UnorderedMap<AssetId, InternalAsset>,
    pub assets_metadata_by_use: LookupMap<UseNumber, Vec<AssetMetadata>>,
}