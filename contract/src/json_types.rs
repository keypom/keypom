use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum JsonDropType {
    Simple,
    NonFungibleToken(JsonNFTData),
    FungibleToken(FTData),
    FunctionCall(FCData),
}

/// Struct to return in views to query for drop info
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonDrop {
    // Drop ID for this drop
    pub drop_id: DropId,
    // owner of this specific drop
    pub owner_id: AccountId,

    // Balance for all keys of this drop. Can be 0 if specified.
    pub deposit_per_use: U128,

    // Every drop must have a type
    pub drop_type: JsonDropType,

    // The drop as a whole can have a config as well
    pub config: Option<DropConfig>,

    // Metadata for the drop
    pub metadata: Option<DropMetadata>,

    // How many claims
    pub registered_uses: u64,

    // Ensure this drop can only be used when the function has the required gas to attach
    pub required_gas: Gas,

    // Keep track of the next nonce to give out to a key
    pub next_key_id: u64,
}

/// NFT Data that is serializable
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonNFTData {
    pub sender_id: AccountId,
    pub contract_id: AccountId,
}

/// FT Data to be passed in by the user
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonFTData {
    pub contract_id: AccountId,
    pub sender_id: AccountId,
    pub balance_per_use: U128,
}

/// Struct to return in views to query for specific data related to an access key.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonKeyInfo {
    // Drop ID for the specific drop
    pub drop_id: DropId,
    pub pk: PublicKey,

    // How many uses this key has left. Once 0 is reached, the key is deleted
    pub remaining_uses: u64,

    // When was the last time the key was used
    pub last_used: u64,

    // How much allowance does the key have left. When the key is deleted, this is refunded to the funder's balance.
    pub allowance: u128,

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
    pub key_use: u64
}