use crate::*;

/// Keep track fungible token data for an access key. This is stored on the contract
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SimpleData {
    // If this is set to true, keys can be created and registered AFTER they've been created (for simple and FC drops only).
    pub lazy_register: Option<bool>,
}