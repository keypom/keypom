use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Serialize, Deserialize, Serializer};
use near_sdk::serde::ser::SerializeStruct;
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, PublicKey, Promise, PromiseOrValue, require, CryptoHash, Gas, Balance};

mod ft_asset;
mod nft_asset;
mod drop_creation;
mod internals;
mod helpers;
mod models;
mod claims;
mod user_balances;
mod drop_deletion;
mod views;
mod nft_keys;

use ft_asset::*;
use nft_asset::*;
use drop_deletion::*;
use internals::*;
use helpers::*;
use models::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Keypom {
    /// Map a drop ID to its internal drop data
    pub drop_by_id: LookupMap<DropId, InternalDrop>,

    /// Get the token ID for any given public key
    pub token_id_by_pk: UnorderedMap<PublicKey, TokenId>,
    /// Keeps track of all the token IDs for a given account
    pub tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,

    /// Keep track of the balances for each user. This is to prepay for drop creations
    pub user_balances: LookupMap<AccountId, Balance>,
    /// Which account should all newly created accounts be sub-accounts of? (i.e `testnet` or `near`)
    pub root_account: AccountId,
}

#[near_bindgen]
impl Keypom {
    #[init]
    pub fn new(root_account: AccountId) -> Self {
        Self {
            drop_by_id: LookupMap::new(StorageKeys::DropById),
            token_id_by_pk: UnorderedMap::new(StorageKeys::TokenIdByPk),
            tokens_per_owner: LookupMap::new(StorageKeys::TokensPerOwner),
            user_balances: LookupMap::new(StorageKeys::UserBalances),
            root_account
        }
    }
}