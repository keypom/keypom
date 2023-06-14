use std::collections::HashMap;

use crate::*;

#[allow(non_camel_case_types)]
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
/// Represents the type of drop that is present in JSON form.
pub enum JsonDropType {
    simple(SimpleData),
    nft(JsonNFTData),
    ft(FTData),
    fc(FCData)
}

/// Struct to return in views to query for drop info
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonDrop {
    /// Drop ID for this drop
    pub drop_id: DropIdJson,
    /// Owner of this specific drop
    pub owner_id: AccountId,

    /// Each time a key is used, how much $NEAR should be sent to the claiming account (can be 0).
    pub deposit_per_use: U128,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub simple: Option<SimpleData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nft: Option<JsonNFTData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ft: Option<FTData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fc: Option<FCData>,

    /// The drop as a whole can have a config that works regardless of the specific drop type.
    pub config: Option<JsonDropConfig>,

    /// Metadata for the drop in the form of stringified JSON. The format is completely up to the
    /// user and there are no standards for format.
    pub metadata: Option<DropMetadata>,

    /// How many key uses are registered and can be used.
    pub registered_uses: u64,

    /// How much Gas should be attached when the key is used. The default is 100 TGas as this is
    /// what's used by the NEAR wallet.
    pub required_gas: Gas,

    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64,
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonDropConfig {
    /// Information about NFT access keys in this drop
    pub nft_key_behaviour: Option<NFTKeyBehaviour>,

    /// How many uses can each key have before it's deleted. If None, default to 1.
    pub uses_per_key: Option<u64>,

    // Any time based configurations
    pub time: Option<TimeConfig>,
    
    // Any usage specific configurations
    pub usage: Option<UsageConfig>,

    pub sale: Option<JsonPublicSaleConfig>,

    /// Override the global root account that sub-accounts will have (near or testnet). This allows
    /// users to create specific drops that can create sub-accounts of a predefined root.
    /// For example, Fayyr could specify a root of `fayyr.near` By which all sub-accounts will then
    /// be `ACCOUNT.fayyr.near`
    pub root_account_id: Option<AccountId>,
}

/// NFT Data that is serializable
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonNFTData {
    pub sender_id: Option<AccountId>,
    pub contract_id: AccountId,
}

/// Public Sale Data that is serializable
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonPublicSaleConfig {
    /// Maximum number of keys that can be added to this drop. If None, there is no max.
    pub max_num_keys: Option<u64>,
 
    /// Amount of $NEAR that the user needs to attach (if they are not the funder) on top of costs. This amount will be
    /// Automatically sent to the funder's balance. If None, the keys are free to the public.
    pub price_per_key: Option<U128>,
 
    #[serde(skip_serializing)]
    /// Which accounts are allowed to add keys?
    pub allowlist: Option<Vec<AccountId>>,
 
    #[serde(skip_serializing)]
    /// Which accounts are NOT allowed to add keys?
    pub blocklist: Option<Vec<AccountId>>,

    /// Should the revenue generated be sent to the funder's account balance or
    /// automatically withdrawn and sent to their NEAR wallet?
    pub auto_withdraw_funds: Option<bool>,

    /// Minimum block timestamp before the public sale starts. If None, keys can be added immediately
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub start: Option<u64>,

    /// Block timestamp dictating the end of the public sale. If None, keys can be added indefinitely
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub end: Option<u64>,
}


/// FT Data to be passed in by the user
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonFTData {
    pub contract_id: AccountId,
    pub sender_id: Option<AccountId>,
    pub balance_per_use: U128,
}

/// Struct to return in views to query for specific data related to an access key.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonKeyInfo {
    // Drop ID for the specific drop
    pub drop_id: DropIdJson,
    pub pk: PublicKey,

    pub token_id: TokenId,

    pub owner_id: AccountId,

    pub approved_account_ids: HashMap<AccountId, u64>,

    pub next_approval_id: u64,

    // Which use is the current key on?
    pub cur_key_use: u64,

    // How many uses this key has left. Once 0 is reached, the key is deleted
    pub remaining_uses: u64,

    // When was the last time the key was used
    pub last_used: u64,

    // How much allowance does the key have left. When the key is deleted, this is refunded to the funder's balance.
    pub allowance: U128,

    // Nonce for the current key.
    pub key_id: u64,
}

/// Struct to pass in for passwords pertaining to a specific use
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonPasswordForUse {
    // What is the password for this use
    pub pw: String,
    // Which use does this pertain to
    pub key_use: u64,
}
