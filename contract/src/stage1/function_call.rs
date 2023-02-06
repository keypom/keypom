use near_sdk::{
    env::{
        promise_batch_create,
        promise_batch_then,
        promise_batch_action_function_call_weight,
    },
    GasWeight
};

use crate::*;

/// Keypom Args struct to be sent to external contracts
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct KeypomArgs {
    pub account_id_field: Option<String>,
    pub drop_id_field: Option<String>,
    pub key_id_field: Option<String>,
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
	    let mut promises: Vec<u64> = vec![];

        for method in methods {
            let keypom_args = KeypomArgs {
                account_id_field: method.account_id_field.clone(),
                drop_id_field: method.drop_id_field.clone(),
                key_id_field: method.key_id_field.clone(),
            };

            let mut final_args = method.args.clone();

            if final_args.contains("\"keypom_args\"") {
                near_sdk::log!(
                    "Keypom Args detected in client args. Returning and decrementing keys"
                );
                return;
            }

            if final_args.len() == 0 {
                final_args = format!(
                    "{{\"keypom_args\":{}}}",
                    near_sdk::serde_json::to_string(&keypom_args).unwrap()
                );
            } else {
                final_args.insert_str(
                    final_args.len() - 1,
                    &format!(
                        ",\"keypom_args\":{}",
                        near_sdk::serde_json::to_string(&keypom_args).unwrap()
                    ),
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

            // start new promise batch or chain with previous promise batch
            let id = if promises.len() == 0 {
                promise_batch_create(&method.receiver_id)
            } else {
                promise_batch_then(
                    promises[promises.len() - 1],
                    &method.receiver_id,
                )
            };
            promises.push(id);

            promise_batch_action_function_call_weight(
                id,
                &method.method_name,
                final_args.as_bytes(),
                method.attached_deposit.0,
                gas,
                GasWeight(1),
            )
        }
    }
}
