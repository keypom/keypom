use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk::{
    env, ext_contract, near_bindgen, promise_result_as_success, require, AccountId, Balance,
    BorshStorageKey, CryptoHash, Gas, PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
    PublicKey,
};

/*
    minimum amount of storage required to store an access key on the contract
*/
const ACCESS_KEY_STORAGE: u128 = 1_000_000_000_000_000_000_000; // 0.001 N

/*
    minimum amount of NEAR that a new account (with longest possible name) must have when created
    If this is less, it will throw a lack balance for state error (assuming you have the same account ID length)
*/
const NEW_ACCOUNT_BASE: u128 = 2_840_000_000_000_000_000_000; // 0.00284 N

/// Indicates there are no attached_deposit for a callback for better readability.
const NO_DEPOSIT: u128 = 0;

/*
    GAS Constants (outlines the minimum to attach. Any unspent GAS will be added according to the weights)
*/
const MIN_GAS_FOR_ON_CLAIM: Gas = Gas(55_000_000_000_000); // 55 TGas

// NFTs
const MIN_GAS_FOR_SIMPLE_NFT_TRANSFER: Gas = Gas(10_000_000_000_000); // 10 TGas
const MIN_GAS_FOR_RESOLVE_TRANSFER: Gas =
    Gas(15_000_000_000_000 + MIN_GAS_FOR_SIMPLE_NFT_TRANSFER.0); // 15 TGas + 10 TGas = 25 TGas

// FTs
// Actual amount of GAS to attach when querying the storage balance bounds. No unspent GAS will be attached on top of this (weight of 0)
const GAS_FOR_STORAGE_BALANCE_BOUNDS: Gas = Gas(10_000_000_000_000); // 10 TGas
const MIN_GAS_FOR_RESOLVE_STORAGE_CHECK: Gas = Gas(25_000_000_000_000); // 25 TGas
const MIN_GAS_FOR_FT_TRANSFER: Gas = Gas(5_000_000_000_000); // 5 TGas
const MIN_GAS_FOR_STORAGE_DEPOSIT: Gas = Gas(5_000_000_000_000); // 5 TGas
const MIN_GAS_FOR_RESOLVE_BATCH: Gas =
    Gas(13_000_000_000_000 + MIN_GAS_FOR_FT_TRANSFER.0 + MIN_GAS_FOR_STORAGE_DEPOSIT.0); // 13 TGas + 5 TGas + 5 TGas = 23 TGas

// Specifies the GAS being attached from the wallet site
const ATTACHED_GAS_FROM_WALLET: Gas = Gas(100_000_000_000_000); // 100 TGas

// Specifies the amount of GAS to attach on top of the FC Gas if executing a regular function call in claim
const GAS_OFFSET_IF_FC_EXECUTE: Gas = Gas(20_000_000_000_000); // 20 TGas

// Actual amount of GAS to attach when creating a new account. No unspent GAS will be attached on top of this (weight of 0)
const GAS_FOR_CREATE_ACCOUNT: Gas = Gas(28_000_000_000_000); // 28 TGas

/// Both methods callable by the function call access key
const ACCESS_KEY_BOTH_METHOD_NAMES: &str = "claim,create_account_and_claim";

/// Only the claim method_name is callable by the access key
const ACCESS_KEY_CLAIM_METHOD_NAME: &str = "claim";

/// Only the create_account_and_claim method_name is callable by the access key
const ACCESS_KEY_CREATE_ACCOUNT_METHOD_NAME: &str = "create_account_and_claim";

/*
    FEES
*/
const DROP_CREATION_FEE: u128 = 1_000_000_000_000_000_000_000_000; // 0.1 N
const KEY_ADDITION_FEE: u128 = 5_000_000_000_000_000_000_000; // 0.005 N

const GAS_FOR_PANIC_OFFSET: Gas = Gas(10_000_000_000_000); // 10 TGas

mod internals;
mod stage1;
mod stage2;
mod stage3;
mod views;

use internals::*;
use stage1::*;
use stage2::*;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    DropIdForPk,
    DropsForId,
    DropIdsForFunder,
    DropIdsForFunderInner { account_id_hash: CryptoHash },
    PksForDrop { account_id_hash: CryptoHash },
    DropMetadata { account_id_hash: CryptoHash },
    TokenIdsForDrop { account_id_hash: CryptoHash },
    FeesPerUser,
    UserBalances,
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct Keypom {
    pub owner_id: AccountId,
    // Which contract is the actual linkdrop deployed to (i.e `testnet` or `near`)
    pub root_account: AccountId,

    // Map each key to a nonce rather than repeating each drop data in memory
    pub drop_id_for_pk: UnorderedMap<PublicKey, DropId>,
    // Map the nonce to a specific drop
    pub drop_for_id: LookupMap<DropId, Drop>,
    // Keep track of the drop ids for each funder for pagination
    pub drop_ids_for_owner: LookupMap<AccountId, UnorderedSet<DropId>>,

    // Fees taken by the contract. One is for creating a drop, the other is for each key in the drop.
    pub drop_fee: u128,
    pub key_fee: u128,
    pub fees_collected: u128,

    // Keep track of fees per each user. Only the owner can edit this.
    pub fees_per_user: LookupMap<AccountId, (u128, u128)>,

    // keep track of the balances for each user. This is to prepay for drop creations
    pub user_balances: LookupMap<AccountId, Balance>,

    // Keep track of a nonce used for the drop IDs
    pub next_drop_id: DropId,

    // Keep track of the price of 1 GAS per 1 yocto
    pub yocto_per_gas: u128,
}

#[near_bindgen]
impl Keypom {
    /// Initialize contract and pass in the desired deployed linkdrop contract (i.e testnet or near)
    #[init]
    pub fn new(root_account: AccountId, owner_id: AccountId) -> Self {
        Self {
            owner_id,
            root_account,
            drop_id_for_pk: UnorderedMap::new(StorageKey::DropIdForPk),
            drop_for_id: LookupMap::new(StorageKey::DropsForId),
            drop_ids_for_owner: LookupMap::new(StorageKey::DropIdsForFunder),
            user_balances: LookupMap::new(StorageKey::UserBalances),
            next_drop_id: 0,
            /*
                FEES
            */
            fees_per_user: LookupMap::new(StorageKey::FeesPerUser),
            drop_fee: DROP_CREATION_FEE,
            key_fee: KEY_ADDITION_FEE,
            fees_collected: 0,
            yocto_per_gas: 100_000_000,
        }
    }
}
