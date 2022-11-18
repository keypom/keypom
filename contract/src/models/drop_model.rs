use crate::*;

/// Drop Metadata should be a string which can be JSON or anything the users want.
pub type DropMetadata = String;

/// ID for a specific drop as a u128.
pub type DropId = u128;
/// JSON equivalent of the Drop ID as a U128.
pub type DropIdJson = U128;
/// Default drop ID as JSON set equal to 0.
pub const DEFAULT_DROP_ID_JSON: U128 = U128(0);
/// If a drop ID is passed in, it MUST be greater than this value (1 billion).
pub const MIN_DROP_ID_PASSED_IN: u128 = 1_000_000_000;

#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
/// Can a key be used to call `claim` or just `create_account_and_claim`? 
pub enum ClaimPermissions {
    claim,
    create_account_and_claim,
}

#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize)]
/// Represents the type of drop that is present. Each type of drop has its own set of data that can optionally be customized.
pub enum DropType {
    simple(SimpleData),
    nft(NFTData),
    ft(FTData),
    fc(FCData),
}

/// Keep track of specific data related to an access key. This allows us to optionally refund funders later.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Drop {
    /// Owner of this specific drop
    pub owner_id: AccountId,
    /// Set of public keys associated with this drop mapped to their specific key information.
    pub pks: UnorderedMap<PublicKey, KeyInfo>,

    /// Each time a key is used, how much $NEAR should be sent to the claiming account (can be 0).
    pub deposit_per_use: u128,
    /// How many key uses are registered and can be used.
    pub registered_uses: u64,

    /// The drop as a whole can have a config that works regardless of the specific drop type.
    pub config: Option<DropConfig>,
    /// Metadata for the drop in the form of stringified JSON. The format is completely up to the
    /// user and there are no standards for format.
    pub metadata: LazyOption<DropMetadata>,
    
    /// Every drop must have a type which contains customizable information.
    pub drop_type: DropType,
    
    /// How much Gas should be attached when the key is used. The default is 100 TGas as this is
    /// what's used by the NEAR wallet.
    pub required_gas: Gas,

    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64,
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DropConfig {
    /// How many uses can each key have before it's deleted. If None, default to 1.
    pub uses_per_key: Option<u64>,

    // Any time based configurations
    pub time: Option<TimeConfig>,
    
    // Any usage specific configurations
    pub usage: Option<UsageConfig>,

    /// Override the global root account that sub-accounts will have (near or testnet). This allows
    /// users to create specific drops that can create sub-accounts of a predefined root.
    /// For example, Fayyr could specify a root of `fayyr.near` By which all sub-accounts will then
    /// be `ACCOUNT.fayyr.near`
    pub root_account_id: Option<AccountId>,
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TimeConfig {
    /// Minimum block timestamp before keys can be used. If None, keys can be used immediately
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub start: Option<u64>,

    /// Block timestamp that keys must be before. If None, keys can be used indefinitely
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub end: Option<u64>,

    /// Time interval between each key use. If None, there is no delay between key uses.
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub throttle: Option<u64>,

    /// Interval of time after the `start_timestamp` that must pass before a key can be used.
    /// If multiple intervals pass, the key can be used multiple times. This has nothing to do
    /// With the throttle timestamp. It only pertains to the start timestamp and the current
    /// timestamp. The last_used timestamp is not taken into account.
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub interval: Option<u64>,
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct UsageConfig {
    /// Can the access key only call the claim method_name? Default to both method_name callable
    pub permissions: Option<ClaimPermissions>,
    /// If claim is called, refund the deposit to the owner's balance. If None, default to false.
    pub refund_deposit: Option<bool>,
    /// Should the drop be automatically deleted when all the keys are used? This is defaulted to false and
    /// Must be overwritten
    pub auto_delete_drop: Option<bool>,
    /// When this drop is deleted and it is the owner's *last* drop, automatically withdraw their balance.
    pub auto_withdraw: Option<bool>,
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize)]
pub struct KeyInfo {
    /// How many uses this key has left. Once 0 is reached, the key is deleted
    pub remaining_uses: u64,

    /// When was the last time the key was used
    pub last_used: u64,

    /// How much allowance does the key have left. When the key is deleted, this is refunded to the funder's balance.
    pub allowance: u128,

    /// Nonce for the current key.
    pub key_id: u64,

    /// Password for each use for this specific key
    pub pw_per_use: Option<UnorderedMap<u64, Vec<u8>>>,

    /// Password for the key regardless of the use
    pub pw_per_key: Option<Vec<u8>>,
}