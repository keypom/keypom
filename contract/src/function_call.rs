use near_sdk::json_types::Base64VecU8;

use crate::*;

/// Keep track of nft data 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    pub receiver: AccountId,
    pub method: String,
    // Optimization
    pub args: Base64VecU8,
    pub deposit: U128,
    pub refund_to_deposit: Option<bool>,
}