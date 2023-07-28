use std::collections::HashSet;

use crate::*;

#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
/// Can a key be used to call `claim` or just `create_account_and_claim`? 
pub enum ClaimPermissions {
    claim,
    create_account_and_claim,
}

/// Keep track of different configuration options for all the uses of a key in a given drop
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DropConfig {
    /// Configurations related to how often keys can be used
    pub time: Option<TimeConfig>,
    
    /// Configurations related to how keys are used
    pub usage: Option<UsageConfig>,

    /// Public sale options for all the keys in this drop
    pub sale: Option<PublicSaleConfig>,

    /// Override the global root account that sub-accounts will have (near or testnet). This allows
    /// users to create specific drops that can create sub-accounts of a predefined root.
    /// For example, Fayyr could specify a root of `fayyr.near` By which all sub-accounts will then
    /// be `ACCOUNT.fayyr.near`
    pub root_account_id: Option<AccountId>,

    /// Should the drop be automatically deleted when all the keys are used? This is defaulted to false and
    /// Must be overwritten
    pub delete_empty_drop: Option<bool>,
}

/// Keep track of different configuration options for a given use
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ConfigForGivenUse {
    /// Configurations related to how often keys can be used
    pub time: Option<TimeConfig>,
    
    /// Configurations related to how keys are used
    pub usage: Option<UsageConfig>,

    /// Override the global root account that sub-accounts will have (near or testnet). This allows
    /// users to create specific drops that can create sub-accounts of a predefined root.
    /// For example, Fayyr could specify a root of `fayyr.near` By which all sub-accounts will then
    /// be `ACCOUNT.fayyr.near`
    pub root_account_id: Option<AccountId>,
}

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

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct PublicSaleConfig {
    /// Maximum number of keys that can be added to this drop. If None, there is no max.
    pub max_num_keys: Option<u64>,
 
    /// Amount of $NEAR that the user needs to attach (if they are not the funder) on top of costs. This amount will be
    /// Automatically sent to the funder's balance. If None, the keys are free to the public.
    pub price_per_key: Option<u128>,
 
    /// Which accounts are allowed to add keys?
    pub allowlist: Option<HashSet<AccountId>>,
 
    /// Which accounts are NOT allowed to add keys?
    pub blocklist: Option<HashSet<AccountId>>,

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

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct UsageConfig {
    /// Can the access key for this use call the claim method_name? Default to both method_name callable
    pub permissions: Option<ClaimPermissions>,
    /// When calling `create_account` on the root account, which keypom args should be attached to the payload.
    pub account_creation_fields: Option<KeypomInjectedArgs>,
}