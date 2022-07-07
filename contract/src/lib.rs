use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::{json};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseResult, PublicKey, PromiseOrValue, promise_result_as_success, CryptoHash,
    require, Balance
};

/* 
    minimum amount of storage required to store an access key on the contract
    1_330_000_000_000_000_000_000 Simple linkdrop: 0.00133 $NEAR
    2_420_000_000_000_000_000_000 NFT Linkdrop: 0.00242 $NEAR
*/
const ACCESS_KEY_STORAGE: u128 = 1_000_000_000_000_000_000_000; // 0.001 N 


/* 
    Base allowance for the access key to cover GAS fees when the account is claimed. This amount is will not be "reserved" on the contract but must be 
    available when GAS is burnt using the access key. The burnt GAS will not be refunded but any unburnt GAS that remains can be refunded.

    If this is lower, wallet will throw the following error:
    Access Key {account_id}:{public_key} does not have enough balance 0.01 for transaction costing 0.018742491841859367297184
*/  
const BASE_ACCESS_KEY_ALLOWANCE: u128 = 18_800_000_000_000_000_000_000; // 0.0188 N
/* 
    minimum amount of NEAR that a new account (with longest possible name) must have when created 
    If this is less, it will throw a lack balance for state error (assuming you have the same account ID length)
*/ 
const NEW_ACCOUNT_BASE: u128 = 2_840_000_000_000_000_000_000; // 0.00284 N

/// Indicates there are no deposit for a callback for better readability.
const NO_DEPOSIT: u128 = 0;

/*
    GAS Constants (outlines the minimum to attach. Any unspent GAS will be added according to the weights)
*/
const MIN_GAS_FOR_ON_CLAIM: Gas = Gas(55_000_000_000_000); // 55 TGas

// NFTs
const MIN_GAS_FOR_SIMPLE_NFT_TRANSFER: Gas = Gas(10_000_000_000_000); // 10 TGas
const MIN_GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(15_000_000_000_000 + MIN_GAS_FOR_SIMPLE_NFT_TRANSFER.0); // 15 TGas + 10 TGas = 25 TGas

// FTs
// Actual amount of GAS to attach when querying the storage balance bounds. No unspent GAS will be attached on top of this (weight of 0)
const GAS_FOR_STORAGE_BALANCE_BOUNDS: Gas = Gas(10_000_000_000_000); // 10 TGas
const MIN_GAS_FOR_RESOLVE_STORAGE_CHECK: Gas = Gas(25_000_000_000_000); // 25 TGas
const MIN_GAS_FOR_FT_TRANSFER: Gas = Gas(5_000_000_000_000); // 5 TGas
const MIN_GAS_FOR_STORAGE_DEPOSIT: Gas = Gas(5_000_000_000_000); // 5 TGas
const MIN_GAS_FOR_RESOLVE_BATCH: Gas = Gas(13_000_000_000_000 + MIN_GAS_FOR_FT_TRANSFER.0 + MIN_GAS_FOR_STORAGE_DEPOSIT.0); // 13 TGas + 5 TGas + 5 TGas = 23 TGas

// Specifies the GAS being attached from the wallet site
const ATTACHED_GAS_FROM_WALLET: Gas = Gas(100_000_000_000_000); // 100 TGas

// Specifies the amount of GAS to attach on top of the FC Gas if executing a regular function call in claim
const GAS_OFFSET_IF_FC_EXECUTE: Gas = Gas(10_000_000_000_000); // 10 TGas

// Actual amount of GAS to attach when creating a new account. No unspent GAS will be attached on top of this (weight of 0)
const GAS_FOR_CREATE_ACCOUNT: Gas = Gas(28_000_000_000_000); // 28 TGas

// Utils
const ONE_GIGGA_GAS: u64 = 1_000_000_000;
const GAS_PRICE: u128 = 100000000;

/// Both methods callable by the function call access key
const ACCESS_KEY_BOTH_METHOD_NAMES: &str = "claim,create_account_and_claim";

/// Only the claim method is callable by the access key
const ACCESS_KEY_CLAIM_METHOD_NAME: &str = "claim";

/*
    FEES
*/
const DROP_CREATION_FEE: u128 = 1_000_000_000_000_000_000_000_000; // 0.1 N 
const KEY_ADDITION_FEE: u128 = 5_000_000_000_000_000_000_000; // 0.005 N 

mod internals;
mod stage1;
mod stage2;
mod stage3;
mod views;

use internals::*;
use stage2::*;
use stage1::*;

pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
    //10^20 yoctoNEAR (1 NEAR would be 10_000). This is to give a precision of 4 decimal places.
    let formatted_near = yocto / 100_000_000_000_000_000_000;
    let near = formatted_near as f64 / 10_000_f64;

    near
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    DropIdForPk,
    DropsForId,
    DropIdsForFunder,
    DropIdsForFunderInner { account_id_hash: CryptoHash },
    PksForDrop { account_id_hash: CryptoHash },
    TokenIdsForDrop { account_id_hash: CryptoHash },
    UserBalances
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct DropZone {
    pub owner_id: AccountId,
    // Which contract is the actual linkdrop deployed to (i.e `testnet` or `near`)
    pub linkdrop_contract: AccountId,
    
    // Map each key to a nonce rather than repeating each drop data in memory
    pub drop_id_for_pk: UnorderedMap<PublicKey, DropId>,
    // Map the nonce to a specific drop
    pub drop_for_id: LookupMap<DropId, Drop>,    
    // Keep track of the drop ids for each funder for pagination
    pub drop_ids_for_funder: LookupMap<AccountId, UnorderedSet<DropId>>,

    // Fees taken by the contract. One is for creating a drop, the other is for each key in the drop.
    pub drop_fee: u128,
    pub key_fee: u128,
    pub fees_collected: u128,

    // keep track of the balances for each user. This is to prepay for drop creations
    pub user_balances: LookupMap<AccountId, Balance>,
    
    // Keep track of a nonce used for the drop IDs
    pub nonce: DropId,

    // Keep track of the price of 1 GAS per 1 yocto
    pub yocto_per_gas: u128
}

#[near_bindgen]
impl DropZone {
    /// Initialize contract and pass in the desired deployed linkdrop contract (i.e testnet or near)
    #[init]
    pub fn new(linkdrop_contract: AccountId, owner_id: AccountId) -> Self {
        Self {
            owner_id,
            linkdrop_contract,
            drop_id_for_pk: UnorderedMap::new(StorageKey::DropIdForPk),
            drop_for_id: LookupMap::new(StorageKey::DropsForId),
            drop_ids_for_funder: LookupMap::new(StorageKey::DropIdsForFunder),
            user_balances: LookupMap::new(StorageKey::UserBalances),
            nonce: 0,
            /*
                FEES
            */
            drop_fee: DROP_CREATION_FEE,
            key_fee: KEY_ADDITION_FEE,
            fees_collected: 0,
            yocto_per_gas: 100000000
        }
    }
}