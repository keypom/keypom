use crate::*;

/// Keep track of info for the method_name to be called
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MethodData {
    // Contract that will be called
    pub receiver_id: AccountId,
    // Method to call on receiver_id contract
    pub method_name: String,
    // Arguments to pass in (stringified JSON)
    pub args: String,
    // Amount of yoctoNEAR to attach along with the call
    pub attached_deposit: U128,
    // Specifies what field the claiming account should go in when calling the function
    // If None, this isn't attached to the args
    pub account_id_field: Option<String>,
    // Specifies what field the drop ID should go in when calling the function.
    // If Some(String), attach drop ID to args. Else, don't attach.
    pub drop_id_field: Option<String>,
    // Specifies what field the key ID should go in when calling the function.
    // If Some(String), attach key ID to args. Else, don't attach.
    pub key_id_field: Option<String>,
}

/// Keep track of optional configurations for the FC data
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCConfig {
    // How much GAS should be attached to the function call if it's a straight execute. Cannot be greater than ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE (90 TGas).
    // This makes it so the keys can only call `claim`
    pub attached_gas: Option<Gas>,
}

/// Keep track of nft data
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    // Vector of optional Vec<MethodData>. If None, no method is called.
    // Drops with more than 1 claim can call a set of different functions each time if Some.
    // If only 1 Vec<MethodData> is passed in for multiple uses, that method data is used for every claim.
    pub methods: Vec<Option<Vec<MethodData>>>,

    // Config for the FC data. If None, all default values are used.
    pub config: Option<FCConfig>,
}