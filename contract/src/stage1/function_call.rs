use near_sdk::GasWeight;

use crate::*;

/// Keypom Args struct to be sent to external contracts
#[derive(Serialize, Deserialize, Debug, BorshDeserialize, BorshSerialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct KeypomArgs {
    pub account_id_field: Option<String>,
    pub drop_id_field: Option<String>,
    pub key_id_field: Option<String>,
    pub funder_id_field: Option<String>,
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
        funder_id: AccountId,
        drop_id: DropId,
        fc_args: Option<Vec<Option<String>>>,
    ) {
        /*
            Function Calls
        */
        let gas = fc_config.and_then(|c| c.attached_gas).unwrap_or(Gas(0));

        for (i, method) in methods.iter().enumerate() {
            let keypom_args = KeypomArgs {
                account_id_field: method.account_id_field.clone(),
                drop_id_field: method.drop_id_field.clone(),
                key_id_field: method.key_id_field.clone(),
                funder_id_field: method.funder_id_field.clone(),
            };

            let mut final_args = method.args.clone();
            near_sdk::log!("Final Args Before Modifications: {}", final_args);

            // Check if user provided args is present and fc_args is some
            if let (Some(rule), Some(user_args)) = (method.user_args_rule.as_ref(), fc_args.clone().and_then(|a| a[i].clone())) {
                match rule {
                    UserArgsRule::AllUser => {
                        final_args = user_args;
                    }
                    UserArgsRule::FunderPreferred => {
                        // Take the final args string and merge the user args into it and overwrite any duplicate keys
                        final_args = merge_string(&user_args, &final_args);
                    }
                    UserArgsRule::UserPreferred => {
                        // Take the final args string and merge the user args into it and overwrite any duplicate keys
                        final_args = merge_string(&final_args, &user_args);
                    }
                }
            }

            if final_args.contains("keypom_args") {
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

            // Add drop_id
            if let Some(field) = method.drop_id_field.as_ref() {
                final_args.insert_str(
                    final_args.len() - 1,
                    &format!(",\"{}\":\"{}\"", field, drop_id),
                );
                near_sdk::log!("Adding drop ID to args {:?}", drop_id,);
            }

            // Add the key_id
            if let Some(field) = method.key_id_field.as_ref() {
                final_args.insert_str(
                    final_args.len() - 1,
                    &format!(",\"{}\":\"{}\"", field, key_id),
                );
                near_sdk::log!("Adding key ID to args {:?}", key_id);
            }

            // Add the funder_id
            if let Some(field) = method.funder_id_field.as_ref() {
                final_args.insert_str(
                    final_args.len() - 1,
                    &format!(",\"{}\":\"{}\"", field, funder_id),
                );
                near_sdk::log!("Adding funder ID to args {:?}", key_id);
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
