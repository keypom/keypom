use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::{json};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseResult, PublicKey, PromiseOrValue, promise_result_as_success, CryptoHash,
};

/* 
    minimum amount of storage required to store an access key on the contract
    1_330_000_000_000_000_000_000 Simple linkdrop: 0.00133 $NEAR
    2_420_000_000_000_000_000_000 NFT Linkdrop: 0.00242 $NEAR
*/
const ACCESS_KEY_STORAGE: u128 = 1_000_000_000_000_000_000_000; // 0.001 N 


/* 
allowance for the access key to cover GAS fees when the account is claimed. This amount is will not be "reserved" on the contract but must be 
    available when GAS is burnt using the access key. The burnt GAS will not be refunded but any unburnt GAS that remains can be refunded.

    If this is lower, wallet will throw the following error:
    Access Key {account_id}:{public_key} does not have enough balance 0.01 for transaction costing 0.018742491841859367297184
*/  
const ACCESS_KEY_ALLOWANCE: u128 = 20_000_000_000_000_000_000_000; // 0.02 N (200 TGas)

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

// Function Calls
const MIN_GAS_FOR_CALLBACK_FUNCTION_CALL: Gas = Gas(30_000_000_000_000); // 30 TGas

// Actual amount of GAS to attach when creating a new account. No unspent GAS will be attached on top of this (weight of 0)
const GAS_FOR_CREATE_ACCOUNT: Gas = Gas(28_000_000_000_000); // 28 TGas

// Utils
const ONE_GIGGA_GAS: u64 = 1_000_000_000;

/// Methods callable by the function call access key
const ACCESS_KEY_METHOD_NAMES: &str = "claim,create_account_and_claim";

mod claim;
mod send;
mod ext_traits;
mod nft;
mod ft;
mod function_call;
mod views;
mod helpers;

use crate::ext_traits::*;
use crate::nft::*;
use crate::ft::*;
use crate::function_call::*;

pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
    //10^20 yoctoNEAR (1 NEAR would be 10_000). This is to give a precision of 4 decimal places.
    let formatted_near = yocto / 100_000_000_000_000_000_000;
    let near = formatted_near as f64 / 10_000_f64;

    near
}

/// Keep track of specific data related to an access key. This allows us to optionally refund funders later. 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountData {
    pub funder_id: AccountId,
    pub balance: U128,
    pub storage_used: U128,

    /*
        EXTRA
    */
    pub cb_id: Option<u64>, //nonce - if set, becomes lookup to all NFT, FT, CD 
    pub cb_data_sent: bool,
    
}

/// Keep track of specific data related to an access key. This allows us to optionally refund funders later. 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct KeyInfo {
    pub pk: Option<PublicKey>,
    pub account_data: Option<AccountData>, 
    pub ft_data: Option<FTData>, 
    pub nft_data: Option<NFTData>, 
    pub fc_data: Option<FCData>
    
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    DataForPublicKey,
    KeysForFunder,
    KeysPerFunderInner { account_id_hash: CryptoHash },
    NFTData,
    FTData,
    FCData,
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct LinkDropProxy {
    pub linkdrop_contract: AccountId,
    pub data_for_pk: UnorderedMap<PublicKey, AccountData>,
    pub keys_for_funder: LookupMap<AccountId, UnorderedSet<PublicKey>>,

    pub nonce: u64,

    /*
        EXTRA
    */
    pub nft: LookupMap<u64, NFTData>,
    pub ft: LookupMap<u64, FTData>,
    pub fc: LookupMap<u64, FCData>
}

#[near_bindgen]
impl LinkDropProxy {
    /// Initialize proxy hub contract and pass in the desired deployed linkdrop contract (i.e testnet or near)
    #[init]
    pub fn new(linkdrop_contract: AccountId) -> Self {
        Self {
            linkdrop_contract,
            data_for_pk: UnorderedMap::new(StorageKey::DataForPublicKey),
            keys_for_funder: LookupMap::new(StorageKey::KeysForFunder),
            nft: LookupMap::new(StorageKey::NFTData),
            ft: LookupMap::new(StorageKey::FTData),
            fc: LookupMap::new(StorageKey::FCData),
            nonce: 0,
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
}