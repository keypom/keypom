use crate::*;

/// Keep track of nft data 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    pub receiver: AccountId,
    pub method: String,
    // Optimization
    pub args: String,
    pub deposit: U128,
    pub refund_to_deposit: Option<bool>,

    // Specifies what field the claiming account should go in when calling the function
    pub claimed_account_field: Option<String>,
}