use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet, LazyOption};
use near_sdk::json_types::U128;
use near_sdk::serde::{Serialize, Deserialize, Serializer};
use near_sdk::serde::ser::SerializeStruct;
use near_sdk::serde_json::json;
use near_sdk::{env, GasWeight, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, PublicKey, Promise, PromiseOrValue, require, CryptoHash, Gas, Balance};


mod assets;
mod drop_creation;
mod internals;
mod helpers;
mod models;
mod drop_claiming;
mod user_balances;
mod drop_deletion;
mod views;
mod owner;
mod nft_keys;

use assets::*;
use nft_keys::*;
use drop_deletion::*;
use drop_creation::*;
use drop_claiming::*;
use internals::*;
use helpers::*;
use models::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Keypom {
    // ------------------------ Owner Only Things ------------------------ //
    /// Owner of the contract that can set configurations such as global freezes etc.
    pub contract_owner_id: AccountId,
    /// Whether or not the contract is frozen and no new drops can be created / keys added.
    pub global_freeze: bool,
    /// Outlines the fees that are charged for every drop and key
    pub fee_structure: KeypomFees,
    /// Total amount of fees available for withdrawal collected overtime.
    pub fees_collected: u128,
    /// Overload the fees for specific users by providing custom fees
    pub fees_per_user: LookupMap<AccountId, KeypomFees>,

    // ------------------------ Drops ------------------------ //
    /// Map a drop ID to its internal drop data
    pub drop_by_id: LookupMap<DropId, InternalDrop>,
    /// Keep track of the drop ids that each funder has created. This is used for view methods.
    pub drop_ids_by_funder: LookupMap<AccountId, UnorderedSet<DropId>>,
    /// Get the token ID for any given public key
    pub token_id_by_pk: UnorderedMap<PublicKey, TokenId>,
    /// Which account should all newly created accounts be sub-accounts of? (i.e `testnet` or `near`)
    pub root_account: AccountId,

    // ------------------------ NFT Keys ------------------------ //
    /// Keeps track of all the token IDs for a given account
    pub tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,
    
    // ------------------------ Utility ------------------------ //
    /// Source metadata extension:
    pub contract_metadata: LazyOption<ContractSourceMetadata>,
    /// Keep track of the balances for each user. This is to prepay for drop creations
    pub user_balances: LookupMap<AccountId, Balance>,
}

#[near_bindgen]
impl Keypom {
    #[init]
    pub fn new(
        root_account: AccountId,
        owner_id: AccountId,
        contract_metadata: ContractSourceMetadata,
    ) -> Self {
        Self {
            contract_owner_id: owner_id,
            global_freeze: false,
            drop_by_id: LookupMap::new(StorageKeys::DropById),
            token_id_by_pk: UnorderedMap::new(StorageKeys::TokenIdByPk),
            tokens_per_owner: LookupMap::new(StorageKeys::TokensPerOwner),
            user_balances: LookupMap::new(StorageKeys::UserBalances),
            root_account,
            fees_per_user: LookupMap::new(StorageKeys::FeesPerUser),
            drop_ids_by_funder: LookupMap::new(StorageKeys::DropIdsByFunder),
            fees_collected: 0,
            fee_structure: KeypomFees {
                per_drop: 0,
                per_key: 0,
            },
            contract_metadata: LazyOption::new(
                StorageKeys::ContractMetadata,
                Some(&contract_metadata),
            ),
        }
    }

    /// Helper function to make sure there isn't a global freeze on the contract
    pub(crate) fn asset_no_global_freeze(&self) {
        if env::predecessor_account_id() != self.contract_owner_id {
            require!(self.global_freeze == false, "Contract is frozen and no new drops or keys can be created");
        }
    }
}