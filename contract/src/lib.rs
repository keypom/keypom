use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::{json};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseResult, PublicKey, PromiseOrValue, promise_result_as_success,
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
const GAS_FOR_STORAGE_BALANCE_BOUNDS: Gas = Gas(10_000_000_000_000); // 10 TGas
const GAS_FOR_RESOLVE_STORAGE_CHECK: Gas = Gas(25_000_000_000_000); // 25 TGas

const GAS_FOR_CREATE_ACCOUNT: Gas = Gas(28_000_000_000_000); // 28 TGas
const GAS_FOR_ON_CLAIM: Gas = Gas(24_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0 + GAS_FOR_SIMPLE_NFT_TRANSFER.0); // 24 TGas + 25 TGas + 10 TGas= 59 TGas 

// NFTs
const GAS_FOR_SIMPLE_NFT_TRANSFER: Gas = Gas(10_000_000_000_000); // 10 TGas
const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(15_000_000_000_000 + GAS_FOR_SIMPLE_NFT_TRANSFER.0); // 15 TGas + 10 TGas = 25 TGas

// FTs
const GAS_FOR_FT_TRANSFER: Gas = Gas(7_500_000_000_000); // 7.5 TGas
const GAS_FOR_STORAGE_DEPOSIT: Gas = Gas(7_500_000_000_000); // 7.5 TGas
const GAS_FOR_RESOLVE_BATCH: Gas = Gas(13_000_000_000_000 + GAS_FOR_FT_TRANSFER.0 + GAS_FOR_STORAGE_DEPOSIT.0); // 10 TGas + 7.5 TGas + 7.5 TGas = 25 TGas

// Function Calls
const GAS_FOR_CALLBACK_FUNCTION_CALL: Gas = Gas(25_000_000_000_000); // 25 TGas

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

/// Defines the type of callback associated with the linkdrop. Either NFT, Fungible Token, or Function call.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum CBType {
    NFT,
    FT,
    FC
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
    pub cb_type: Option<CBType>, //nonce - if set, becomes lookup to all NFT, FT, CD 
    pub cb_id: Option<u64>, //nonce - if set, becomes lookup to all NFT, FT, CD 
    pub cb_data_sent: bool,
    
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Accounts,
    NFTData,
    FTData,
    FCData,
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct LinkDropProxy {
    pub linkdrop_contract: AccountId,
    pub accounts: LookupMap<PublicKey, AccountData>,

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
            accounts: LookupMap::new(StorageKey::Accounts),
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

    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let account_data = self.accounts
            .get(&key)
            .expect("Key missing");
        (account_data.balance.0).into()
    }

    /// Returns the data corresponding to a specific key
    pub fn get_key_information(
        &self, 
        key: PublicKey
    ) -> (AccountData, Option<NFTData>, Option<FTData>, Option<FCData>) {
        // By default, every key should have account data
        let account_data = self.accounts
            .get(&key)
            .expect("Key missing");
        
        // If the linkdrop has a callback ID, return the specific callback info. Otherwise, return only account data. 
        if let Some(nonce) = account_data.cb_id {
            let cb_type = account_data.clone().cb_type.unwrap();

            // Check for the specific callback type and return the info.
            match cb_type {
                CBType::NFT => {
                    return (
                        account_data,
                        self.nft.get(&nonce),
                        None,
                        None
                    )
                },
                CBType::FT => {
                    return (
                        account_data,
                        None,
                        self.ft.get(&nonce),
                        None
                    )
                },
                CBType::FC => {
                    return (
                        account_data,
                        None,
                        None,
                        self.fc.get(&nonce)
                    )
                }
            }
        } else {
            return (
                account_data,
                None,
                None,
                None
            )
        }
    }

    /// Returns the current nonce on the contract
    pub fn get_nonce(&self) -> u64 {
        self.nonce
    }   
}