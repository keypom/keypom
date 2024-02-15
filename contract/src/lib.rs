use std::collections::HashMap;

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde::ser::SerializeStruct;
use near_sdk::serde::{Deserialize, Serialize, Serializer};
use near_sdk::serde_json::json;
use near_sdk::{
    env, near_bindgen, require, AccountId, Allowance, BorshStorageKey, CryptoHash, Gas, GasWeight,
    NearToken, PanicOnDefault, Promise, PromiseOrValue, PublicKey,
};

mod assets;
mod drop_claiming;
mod drop_creation;
mod drop_deletion;
mod helpers;
mod internals;
mod models;
mod nft_keys;
mod owner;
mod user_balances;
mod views;

use assets::*;
use drop_claiming::*;
use drop_creation::*;
use drop_deletion::*;
use helpers::*;
use internals::*;
use models::*;
use nft_keys::*;

pub type Balance = u128;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
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
    /// Key used to sign transactions for the contract
    pub signing_pk: PublicKey,
    pub signing_sk: String,
    pub message: String,

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
    /// Keep track of the balances for each user. This is to prepay for drop creations
    pub user_balances: LookupMap<AccountId, Balance>,
}

#[near_bindgen]
impl Keypom {
    #[init]
    pub fn new(
        root_account: AccountId,
        owner_id: AccountId,
        signing_pk: PublicKey,
        signing_sk: String,
        message: String,
    ) -> Self {
        Promise::new(env::current_account_id()).add_access_key_allowance(
            signing_pk.clone(),
            Allowance::Unlimited,
            env::current_account_id(),
            GLOBAL_KEY_METHOD_NAMES.to_string(),
        );

        env::log_str(format!("Signing PK: {:?}, SK: {}", signing_pk, signing_sk).as_str());

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
            signing_pk,
            signing_sk,
            message,
        }
    }

    /// Helper function to make sure there isn't a global freeze on the contract
    pub(crate) fn assert_no_global_freeze(&self) {
        if env::predecessor_account_id() != self.contract_owner_id {
            require!(
                !self.global_freeze,
                "Contract is frozen and no new drops or keys can be created"
            );
        }
    }

    pub(crate) fn verify_signature(&mut self, signature: Base64VecU8, pk: PublicKey) -> bool {
        self.assert_contract_key();
        near_sdk::log!(
            "Verifying PK: {}: {:?}",
            serde_json::to_string(&pk).unwrap(),
            pk
        );
        let token_id = self
            .token_id_by_pk
            .get(&pk)
            .expect("No drop ID found for PK");

        let (drop_id, _) = parse_token_id(&token_id).unwrap();
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop
            .key_info_by_token_id
            .get(&token_id)
            .expect("Key not found");

        let expected_message = format!("{}{}", self.message, key_info.message_nonce);

        let pk_bytes = pk_to_32_byte_array(&pk).unwrap();
        let sig_bytes = vec_to_64_byte_array(signature.into()).unwrap();
        let is_valid = env::ed25519_verify(&sig_bytes, expected_message.as_bytes(), pk_bytes);

        // Only increment the nonce if the signature is valid.
        // Otherwise, someone could pass in a different public key and increment their nonce
        // without needing their secret key
        if is_valid {
            key_info.message_nonce += 1;
            drop.key_info_by_token_id.insert(&token_id, &key_info);
            self.drop_by_id.insert(&drop_id, &drop);
        }

        is_valid
    }
}
