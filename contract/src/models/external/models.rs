use crate::*;

/// When creating a drop, assets can either be specified on a per use basis or for all uses
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum ExtAssetData {
    AssetsPerUse(Vec<ExtAssetDataForGivenUse>),
    AssetsForAllUses(ExtAssetDataForAllUses)
}

/// Outlines the different asset types that can be used in drops. This is the external version of `InternalAsset`
/// And represents the data that is passed into and out of the Keypom contract
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum ExtAsset {
    FTAsset(ExtFTData),
    NearAsset(ExtNEARData),
    NFTAsset(ExtNFTData),
    FCAsset(FCData)
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtNEARData {
    /// How much $NEAR should be transferred as part of the asset claim
    pub yoctonear: U128
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
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
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtNFTData {
    /// The contract that the non fungible tokens originate from
    pub nft_contract_id: AccountId
}

/// Drop data being returned from view calls from Keypom
#[derive(BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtDrop {
    pub asset_data: ExtAssetData,

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

/// For any given use of a key, there's a set of assets and also a config that is shared
/// For all keys in a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtAssetDataForGivenUse {
    /// Which assets should be present for this use
    pub assets: Vec<Option<ExtAsset>>,
    /// What config should be used for this use
    pub config: Option<ConfigForGivenUse>
}

/// Data for each key coming in (public key, password, metadata, owner etc.)
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtKeyData {
    /// What is the public key?
    pub public_key: PublicKey,
    /// A map outlining what the password should be for any given use.
    /// The password here should be a double hash and when claim is called,
    /// The user arguments are hashed and compared to the password here (i.e user passes in single hash)
    pub password_by_use: Option<HashMap<UseNumber, String>>,
    /// Metadata for the given key represented as a string. Most often, this will be JSON stringified.
    pub metadata: Option<String>,
    /// What account ID owns the given key (if any)
    pub key_owner: Option<AccountId>
}

/// Optional aata for the drop such as configs, metadata etc.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtDropData {
    /// Any configurations for the drop such as public sale info etc.
    /// This will be applied to ALL uses & keys in the drop unless a config
    /// Is present for the current key use (which will override this)
    pub config: Option<DropConfig>,
    /// Metadata for the given drop represented as a string. Most often, this will be JSON stringified.
    pub metadata: Option<DropMetadata>,
    /// Configurations for all the NFT keys in this drop. This contains info about royalties and metadata
    /// That each key will inherit
    pub nft_keys_config: Option<NFTKeyConfigurations>,
}