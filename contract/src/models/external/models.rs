use crate::*;

/// Outlines the different asset types that can be used in drops. This is the external version of `InternalAsset`
/// And represents the data that is passed into and out of the Keypom contract
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum ExtAsset {
    FTAsset(ExtFTData),
    NearAsset(ExtNEARData),
    NFTAsset(ExtNFTData),
    FCAsset(FCData)
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtNEARData {
    /// How much $NEAR should be transferred as part of the asset claim
    pub yoctonear: U128
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
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
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
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
    pub fc_asset_data: Vec<FCData>,

    pub metadata: Option<DropMetadata>
}

/// If the user wishes to specify a set of assets that is repeated across many uses, they can use
/// This struct rather than pasting duplicate data when calling `create_drop`
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtAssetDataForAllUses {
    /// Which assets should be present for each use
    pub assets: Vec<Option<ExtAsset>>,
    /// How many uses are there for this drop?
    pub num_uses: UseNumber,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetDataForGivenUse {
    /// Which assets should be present for this use
    pub assets: Vec<Option<ExtAsset>>,
    /// What config should be used for this use
    pub config: Option<ConfigForGivenUse>
}

/// Specifies exactly what assets are contained in every given use for a drop
pub type ExtAssetDataPerUse = HashMap<UseNumber, AssetDataForGivenUse>;

pub type ExtConfig = Option<bool>;