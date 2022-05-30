use crate::*;

/// Keep track of nft data 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    pub receiver: AccountId,
    pub method: String,
    pub args: String,
    pub deposit: U128
}