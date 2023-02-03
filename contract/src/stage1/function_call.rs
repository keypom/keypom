use near_sdk::GasWeight;

use serde_json::{Value, from_str, to_string};

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
            let mut final_args = method.args.clone();
            if final_args.len() == 0 {
                final_args = "{}".to_string();
            }
            
            let try_json: Result<Value, _> = from_str(&final_args);
            if try_json.is_err() {
                near_sdk::log!(
                    "Cannot cast args to JSON. Returning and decrementing keys"
                );
                return;
            }

            let mut final_args_json = try_json.unwrap();

            // Check if user provided args is present and fc_args is some
            if let (Some(rule), Some(user_args_str)) = (method.user_args_rule.as_ref(), fc_args.clone().and_then(|a| a[i].clone())) {
                let try_user_json: Result<Value, _> = from_str(&user_args_str);
                if try_user_json.is_ok() {
                    let mut user_args_json = try_user_json.unwrap();
                    
                    match rule {
                        UserArgsRule::AllUser => {
                            final_args_json = user_args_json;
                        }
                        UserArgsRule::FunderPreferred => {
                            // Take the final args string and merge the user args into it and overwrite any duplicate keys
                            merge_json(&mut user_args_json, &final_args_json);
                            final_args_json = user_args_json;
                        }
                        UserArgsRule::UserPreferred => {
                            // Take the final args string and merge the user args into it and overwrite any duplicate keys
                            merge_json(&mut final_args_json, &user_args_json);
                        }
                    }
                } else {
                    return Err("Cannot cast user provided args to JSON. Returning and decrementing keys".to_string());
                }
            }

            final_args_json["keypom_args"] = json!({
                "account_id_field": method.account_id_field.clone(),
                "drop_id_field": method.drop_id_field.clone(),
                "key_id_field": method.key_id_field.clone(),
                "funder_id_field": method.funder_id_field.clone(),
            });

            // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
            if let Some(field) = method.account_id_field.as_ref() {
                final_args_json[field] = json!(account_id);
                near_sdk::log!(
                    "Adding claimed account ID to specified field: {:?}",
                    method.account_id_field,
                );
            }

            // Add drop_id
            if let Some(field) = method.drop_id_field.as_ref() {
                final_args_json[field] = json!(drop_id.to_string());
                near_sdk::log!("Adding drop ID to args {:?}", drop_id);
            }

            // Add the key_id
            if let Some(field) = method.key_id_field.as_ref() {
                final_args_json[field] = json!(key_id.to_string());
                near_sdk::log!("Adding key ID to args {:?}", key_id);
            }

            // Add the funder_id
            if let Some(field) = method.funder_id_field.as_ref() {
                final_args_json[field] = json!(funder_id);
                near_sdk::log!("Adding funder ID to args {:?}", funder_id);
            }

            if to_string(&final_args_json).is_err() {
                near_sdk::log!("Cannot cast final args to JSON. Returning and decrementing keys");
                return;
            }

            final_args = to_string(&final_args_json).unwrap();
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
