use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::json_types::U128;
use near_sdk::serde::{Serialize, Deserialize, Serializer};
use near_sdk::serde::ser::SerializeStruct;
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, PublicKey, Promise, PromiseOrValue, require, CryptoHash, Gas, Balance};

mod ft_asset;
mod drop_creation;
mod internals;
mod helpers;
mod models;
mod claims;
mod user_balances;
mod drop_deletion;
mod views;

use ft_asset::*;
use drop_deletion::*;
use internals::*;
use helpers::*;
use models::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Keypom {
    /// Map a drop ID to its internal drop data
    drop_by_id: LookupMap<DropId, InternalDrop>,
    /// Map of each key to its respective drop ID. This is much more efficient than repeating the
    /// Drop data for every single key.
    drop_id_for_pk: UnorderedMap<PublicKey, DropId>,
    /// Keep track of the balances for each user. This is to prepay for drop creations
    user_balances: LookupMap<AccountId, Balance>,
    /// Which account should all newly created accounts be sub-accounts of? (i.e `testnet` or `near`)
    pub root_account: AccountId,
}

#[near_bindgen]
impl Keypom {
    #[init]
    pub fn new(root_account: AccountId) -> Self {
        Self {
            drop_by_id: LookupMap::new(StorageKeys::DropById),
            drop_id_for_pk: UnorderedMap::new(StorageKeys::DropIdByPk),
            user_balances: LookupMap::new(StorageKeys::UserBalances),
            root_account
        }
    }
}