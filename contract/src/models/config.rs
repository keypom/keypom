use crate::*;
use std::collections::HashSet;

#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
/// Can a key be used to call `claim` or just `create_account_and_claim`? 
pub enum ClaimPermissions {
    claim,
    create_account_and_claim,
}

/// Keep track of different configuration options for all the uses of a key in a given drop
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct UseConfig {
    /// Configurations related to how often keys can be used
    pub time: Option<TimeConfig>,
    
    /// Can the access key for this use call the claim method_name? Default to both method_name callable
    pub permissions: Option<ClaimPermissions>,
    
    /// When calling `create_account` on the root account, which keypom args should be attached to the payload.
    pub account_creation_keypom_args: Option<KeypomInjectedArgs>,

    /// Override the global root account that sub-accounts will have (near or testnet). This allows
    /// users to create specific drops that can create sub-accounts of a predefined root.
    /// For example, Fayyr could specify a root of `fayyr.near` By which all sub-accounts will then
    /// be `ACCOUNT.fayyr.near`
    pub root_account_id: Option<AccountId>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
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

/// Optional configurations for the drop such as metadata, deleting empty drops etc.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DropConfig {
    /// Metadata for the given drop represented as a string. Most often, this will be JSON stringified.
    pub metadata: Option<DropMetadata>,
    /// Configurations for all the NFT keys in this drop. This contains info about royalties and metadata
    /// That each key will inherit
    pub nft_keys_config: Option<NFTKeyConfigurations>,

    /// Which users can add keys to the drop. The public sale config was moved out of the Keypom contract
    /// And now should be deployed on its own proxy contract that in turn performs any necessary sale logic
    /// And then fires a cross contract call to the Keypom contract to add keys
    pub add_key_allowlist: Option<HashSet<AccountId>>,

    /// Should the drop be automatically deleted when all the keys are used? This is defaulted to true and
    /// Must be overwritten
    pub delete_empty_drop: Option<bool>,

    /// How much extra allowance should be given to each key in the drop?
    /// This allows keys to be used for extra functionalities such as `nft_transfer`, `nft_approve`, etc.
    pub extra_allowance_per_key: Option<U128>
}