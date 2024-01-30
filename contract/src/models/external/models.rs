use crate::*;

/// Represents the asset data including configs for a set amount of uses.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct ExtAssetDataForUses {
    /// How many uses does this asset data apply to?
    pub uses: UseNumber,
    /// Which assets should be present for these uses
    pub assets: Vec<Option<ExtAsset>>,
    /// Any configurations for this set of uses
    pub config: Option<UseConfig>,
}

/// Outlines the different asset types that can be used in drops. This is the external version of `InternalAsset`
/// And represents the data that is passed into and out of the Keypom contract
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
#[serde(untagged)]
pub enum ExtAsset {
    FTAsset(ExtFTData),
    NearAsset(ExtNEARData),
    NFTAsset(ExtNFTData),
    FCAsset(Vec<MethodData>),
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct ExtNEARData {
    /// How much $NEAR should be transferred as part of the asset claim
    pub yoctonear: U128,
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct ExtFTData {
    /// The contract that the fungible tokens originate from
    pub ft_contract_id: AccountId,
    /// How much $NEAR (in yocto) it costs to register a new user on the fungible token contract
    pub registration_cost: U128,
    /// How many fungible tokens (in their smallest indivisible unit) should be transferred as part of the asset claim
    pub ft_amount: U128,
}

/// Data going into or out of the Keypom contract representing the presence of non-fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]

pub struct ExtNFTData {
    /// The contract that the non fungible tokens originate from
    pub nft_contract_id: AccountId,
}

/// Drop data being returned from view calls from Keypom
#[derive(BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]

pub struct ExtDrop {
    /// ID for this specific drop
    pub drop_id: DropId,
    /// Account ID who funded / owns the rights to this specific drop
    pub funder_id: AccountId,
    /// What is the maximum number of uses a given key can have in the drop?
    pub max_key_uses: UseNumber,

    pub asset_data: Vec<ExtAssetDataForUses>,

    pub nft_asset_data: Vec<InternalNFTData>,
    pub ft_asset_data: Vec<InternalFTData>,

    /// Keep track of different configuration options for all the uses of a key in a given drop
    pub drop_config: Option<DropConfig>,

    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64,
}

/// Data for each key coming in (public key, password, metadata, owner etc.)
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]

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
    pub key_owner: Option<AccountId>,
}

