use crate::*;

/// Keep track of nft data 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    // Contract that will be called
    pub receiver: AccountId,
    // Method to call on receiver contract
    pub method: String,
    // Arguments to pass in (stringified JSON)
    pub args: String,
    // Amount of yoctoNEAR to attach along with the call
    pub deposit: U128,
    // Should the refund that normally goes to the funder be attached alongside the deposit?
    pub refund_to_deposit: Option<bool>,
    // Specifies what field the claiming account should go in when calling the function
    pub claimed_account_field: Option<String>,
}