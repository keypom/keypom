use crate::*;

/// Outlines the different asset types that can be used in drops. This is the external version of `InternalAsset`
/// And represents the data that is passed into and out of the Keypom contract
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum ExtAsset {
    FTAsset(ExtFTData),
    NearAsset(ExtNEARData),
    NFTAsset(ExtNFTData),
}

pub(crate) fn ext_asset_to_internal(ext_asset: Option<&ExtAsset>) -> InternalAsset {
    if let Some(asset) = ext_asset {
        return asset.to_internal_asset();
    }

    return InternalAsset::none;
}

impl ExtAsset {
    /// Convert an `ExtAsset` into an `InternalAsset`
    pub fn to_internal_asset(&self) -> InternalAsset {
        match self {
            ExtAsset::FTAsset(ft_data) => InternalAsset::ft(InternalFTData::new(
                ft_data.ft_contract_id.clone(),
                ft_data.registration_cost.into(),
            )),
            ExtAsset::NFTAsset(nft_data) => InternalAsset::nft(InternalNFTData::new(
                nft_data.nft_contract_id.clone(),
            )),
            ExtAsset::NearAsset(_) => InternalAsset::near,
        }
    }

    /// Standard function to check how many tokens a given asset transfers per use
    pub fn get_tokens_per_use(&self) -> U128 {
        match self {
            ExtAsset::FTAsset(ft_data) => ft_data.ft_amount.into(),
            ExtAsset::NFTAsset(_) => U128(0),
            ExtAsset::NearAsset(near_data) => near_data.yoctonear.into()
        }
    }

    /// Standard function to query for the asset ID of the current external asset
    pub fn get_asset_id(&self) -> AssetId {
        match self {
            ExtAsset::FTAsset(ft_data) => ft_data.ft_contract_id.to_string(),
            ExtAsset::NFTAsset(nft_data) => nft_data.nft_contract_id.to_string(),
            ExtAsset::NearAsset(_) => NEAR_ASSET_ID.to_string(),
        }
    }
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtNEARData {
    /// How much $NEAR should be transferred as part of the asset claim
    pub yoctonear: U128
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtFTData {
    /// The contract that the fungible tokens originate from
    pub ft_contract_id: AccountId,
    /// How much $NEAR (in yocto) it costs to register a new user on the fungible token contract
    pub registration_cost: U128,
    /// How many fungible tokens (in their smallest indivisible unit) should be transferred as part of the asset claim
    pub ft_amount: U128
}

/// Data going into or out of the Keypom contract representing the presence of non-fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtNFTData {
    /// The contract that the non fungible tokens originate from
    pub nft_contract_id: AccountId
}

/// Drop data being returned from view calls from Keypom
#[derive(BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtDrop {
    pub assets_per_use: HashMap<UseNumber, Vec<Option<ExtAsset>>>,

    pub nft_asset_data: Vec<InternalNFTData>,
    pub ft_asset_data: Vec<InternalFTData>,

    pub metadata: Option<DropMetadata>
}