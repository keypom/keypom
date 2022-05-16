use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize};
use near_sdk::serde_json::{json};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseResult, PublicKey, PromiseOrValue, promise_result_as_success,
};

/* 
    minimum amount of storage required to cover:
    - storing access key on the contract
    - storing pub key and account data in the map
    Some of this can be refunded once the account is claimed.
*/ 
const STORAGE_ALLOWANCE: u128 = 5_000_000_000_000_000_000_000; // 0.005 N 
/* 
    allowance for the access key to cover GAS fees when the account is claimed. This amount is will not be "reserved" on the contract but must be 
    available when GAS is burnt using the access key. The burnt GAS will not be refunded but any unburnt GAS that remains can be refunded.

    If this is lower, wallet will throw the following error:
    Access Key {account_id}:{public_key} does not have enough balance 0.01 for transaction costing 0.018742491841859367297184
*/  
const ACCESS_KEY_ALLOWANCE: u128 = 20_000_000_000_000_000_000_000; // 0.02 N 
/* 
    minimum amount of NEAR that a new account (with longest possible name) must have when created 
    If this is less, it will throw a lack balance for state error (assuming you have the same account ID length)
*/ 
const NEW_ACCOUNT_BASE: u128 = 2_840_000_000_000_000_000_000; // 0.00284 N

/// Indicates there are no deposit for a callback for better readability.
const NO_DEPOSIT: u128 = 0;

// Defaulting burnt GAS to be 100 TGas (0.01 $NEAR)
const BURNT_GAS: u128 = 10_000_000_000_000_000_000_000;

/*
    GAS Constants
*/
const GAS_FOR_SIMPLE_NFT_TRANSFER: Gas = Gas(10_000_000_000_000); // 10 TGas
const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(15_000_000_000_000 + GAS_FOR_SIMPLE_NFT_TRANSFER.0); // 15 TGas + 10 TGas = 25 TGas

const GAS_FOR_ON_CLAIM: Gas = Gas(24_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0 + GAS_FOR_SIMPLE_NFT_TRANSFER.0); // 24 TGas + 25 TGas + 10 TGas= 59 TGas 
const GAS_FOR_CREATE_ACCOUNT: Gas = Gas(28_000_000_000_000); // 28 TGas

const GAS_FOR_STORAGE_BALANCE_BOUNDS: Gas = Gas(10_000_000_000_000); // 10 TGas
const GAS_FOR_RESOLVE_STORAGE_CHECK: Gas = Gas(25_000_000_000_000); // 25 TGas

const GAS_FOR_FT_TRANSFER: Gas = Gas(7_500_000_000_000); // 7.5 TGas
const GAS_FOR_STORAGE_DEPOSIT: Gas = Gas(7_500_000_000_000); // 7.5 TGas
const GAS_FOR_RESOLVE_BATCH: Gas = Gas(13_000_000_000_000 + GAS_FOR_FT_TRANSFER.0 + GAS_FOR_STORAGE_DEPOSIT.0); // 10 TGas + 7.5 TGas + 7.5 TGas = 25 TGas

const ONE_GIGGA_GAS: u64 = 1_000_000_000;


/// Methods callable by the function call access key
const ACCESS_KEY_METHOD_NAMES: &str = "claim,create_account_and_claim";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Accounts,
}
/// Keep track of specific data related to an access key. This allows us to optionally refund funders later. 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct AccountData {
    pub funder_id: AccountId,
    pub balance: U128,
    pub token_sender: Option<AccountId>,
    pub token_contract: Option<AccountId>,
    pub nft_id: Option<String>,
    pub ft_balance: Option<U128>,
    pub ft_storage: Option<U128>,
}

mod claim;
mod send;
mod ext_traits;
mod nft;
mod ft;

use crate::ext_traits::*;

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct LinkDropProxy {
    pub linkdrop_contract: AccountId,
    pub accounts: LookupMap<PublicKey, AccountData>,
}

#[near_bindgen]
impl LinkDropProxy {
    /// Initialize proxy hub contract and pass in the desired deployed linkdrop contract (i.e testnet or near)
    #[init]
    pub fn new(linkdrop_contract: AccountId) -> Self {
        Self {
            linkdrop_contract,
            accounts: LookupMap::new(StorageKey::Accounts),
        }
    }

    /// Set the desired linkdrop contract to interact with
	pub fn set_contract(&mut self, linkdrop_contract: AccountId) {
		assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
		self.linkdrop_contract = linkdrop_contract;
	}

    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let account_data = self.accounts
            .get(&key)
            .expect("Key missing");
        (account_data.balance.0).into()
    }
}