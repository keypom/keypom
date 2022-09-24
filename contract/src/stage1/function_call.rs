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
    // If only 1 Vec<MethodData> is passed in for multiple claims, that method data is used for every claim.
    pub methods: Vec<Option<Vec<MethodData>>>,

    // Config for the FC data. If None, all default values are used.
    pub config: Option<FCConfig>,
}

#[near_bindgen]
impl Keypom {
    // Internal method_name for transfer NFTs.
    pub(crate) fn internal_fc_execute(
        &mut self,
        methods: &Vec<MethodData>,
        fc_config: Option<FCConfig>,
        key_id: u64,
        account_id: AccountId,
        drop_id: DropId,
    ) {
        /*
            Function Calls
        */
        let gas = fc_config.and_then(|c| c.attached_gas).unwrap_or(Gas(0));

        for method in methods {
            // Get binary representation of whether or not account ID field, drop ID field, and key ID field are present
            let injected_fields = 2u8.pow(0) * method.account_id_field.is_some() as u8
                + 2u8.pow(1) * method.drop_id_field.is_some() as u8
                + 2u8.pow(2) * method.key_id_field.is_some() as u8;

            let mut final_args = method.args.clone();

            if final_args.contains("\"injected_fields\"") {
                near_sdk::log!(
                    "Injected fields detected in client args. Returning and decrementing keys"
                );
                return;
            }

            if final_args.len() == 0 {
                final_args = format!("{{\"injected_fields\":\"{}\"}}", injected_fields);
            } else {
                final_args.insert_str(
                    final_args.len() - 1,
                    &format!(",\"injected_fields\":\"{}\"", injected_fields),
                );
            }

            // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
            if let Some(field) = method.account_id_field.as_ref() {
                final_args.insert_str(
                    final_args.len() - 1,
                    &format!(",\"{}\":\"{}\"", field, account_id),
                );
                near_sdk::log!(
                    "Adding claimed account ID to specified field: {:?}",
                    method.account_id_field,
                );
            }

            // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
            if let Some(field) = method.drop_id_field.as_ref() {
                final_args.insert_str(
                    final_args.len() - 1,
                    &format!(",\"{}\":\"{}\"", field, drop_id),
                );
                near_sdk::log!("Adding drop ID to args {:?}", drop_id,);
            }

            // Add the key ID as part of the args to the function call
            if let Some(field) = method.key_id_field.as_ref() {
                final_args.insert_str(
                    final_args.len() - 1,
                    &format!(",\"{}\":\"{}\"", field, key_id),
                );
                near_sdk::log!("Adding key ID to args {:?}", key_id);
            }

            near_sdk::log!("Final args {:?}", final_args);

            // Call function with the min GAS and attached_deposit. all unspent GAS will be added on top
            Promise::new(method.receiver_id.clone()).function_call_weight(
                method.method_name.clone(),
                final_args.as_bytes().to_vec(),
                // The claim is successful so attach the amount to refund to the attached_deposit instead of refunding the funder.
                method.attached_deposit.0,
                gas,
                GasWeight(1),
            );
        }
    }
}
