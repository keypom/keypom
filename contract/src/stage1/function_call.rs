use near_sdk::GasWeight;

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
}

/// Keep track of optional configurations for the FC data
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCConfig {
    // Specifies what field the claiming account should go in when calling the function
    // If None, this isn't attached to the args
    pub account_id_field: Option<String>,

    // Specifies what field the drop ID should go in when calling the function.
    // If Some(String), attach drop ID to args. Else, don't attach.
    pub drop_id_field: Option<String>,

    // Specifies what field the key ID should go in when calling the function.
    // If Some(String), attach key ID to args. Else, don't attach.
    pub key_id_field: Option<String>,

    // How much GAS should be attached to the function call if it's a straight execute. Cannot be greater than ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE (90 TGas).
    // This makes it so the keys can only call `claim`
    pub attached_gas: Option<Gas>,
}

/// Keep track of nft data
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    // Vector of optional method_name data. If None, no method_name is called.
    // Drops with more than 1 claim can call different functions each time if Some.
    // If only 1 method_name is passed in for multiple claims, that method_name is used for every claim.
    pub methods: Vec<Option<MethodData>>,

    // Config for the FC data. If None, all default values are used.
    pub config: Option<FCConfig>,
}

#[near_bindgen]
impl Keypom {
    // Internal method_name for transfer NFTs.
    pub(crate) fn internal_fc_execute(
        &mut self,
        methods: &MethodData,
        fc_config: Option<FCConfig>,
        key_id: u64,
        account_id: AccountId,
        drop_id: DropId,
    ) {
        /*
            Function Calls
        */
        let mut final_args = methods.args.clone();

        // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
        if let Some(account_field) = fc_config.clone().and_then(|c| c.account_id_field) {
            final_args.insert_str(
                final_args.len() - 1,
                &format!(",\"{}\":\"{}\"", account_field, account_id),
            );
            near_sdk::log!(
                "Adding claimed account ID to specified field: {:?}",
                account_field,
            );
        }

        // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
        if let Some(drop_id_field) = fc_config.clone().and_then(|c| c.drop_id_field) {
            final_args.insert_str(
                final_args.len() - 1,
                &format!(",\"{}\":\"{}\"", drop_id_field, drop_id),
            );
            near_sdk::log!("Adding drop ID to args {:?}", drop_id,);
        }

        // Add the key ID as part of the args to the function call
        if let Some(key_id_field) = fc_config.clone().and_then(|c| c.key_id_field) {
            final_args.insert_str(
                final_args.len() - 1,
                &format!(",\"{}\":\"{}\"", key_id_field, key_id),
            );
            near_sdk::log!("Adding key ID to args {:?}", key_id);
        }

        // Call function with the min GAS and attached_deposit. all unspent GAS will be added on top
        Promise::new(methods.receiver_id.clone()).function_call_weight(
            methods.method_name.clone(),
            final_args.as_bytes().to_vec(),
            // The claim is successful so attach the amount to refund to the attached_deposit instead of refunding the funder.
            methods.attached_deposit.0,
            fc_config.and_then(|c| c.attached_gas).unwrap_or(Gas(0)),
            GasWeight(1),
        );
    }
}
