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

/// Keep track of nft data
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonNFTData {
    pub sender_id: AccountId,
    pub contract_id: AccountId,
}

/// Struct to return in views to query for specific data related to an access key.
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonKeyInfo {
    // Drop ID for the specific drop
    pub drop_id: DropId,
    pub pk: PublicKey,
    pub key_info: KeyInfo,
}