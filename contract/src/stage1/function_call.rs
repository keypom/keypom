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

pub(crate) fn add_keypom_field(
    output_args: &mut Value,
    field: &String,
    value: String
) -> bool {
    let temp_option = output_args.as_object_mut();
    if temp_option.is_none() {
        return false
    } 
    
    // Temp is a map now
    let mut temp = temp_option.unwrap();
    let split = field.split(".");
    let last_el = split.clone().count() - 1;
    
    // near_sdk::log!("last_el: {}", last_el);
    for (i, e) in split.enumerate() {
        // near_sdk::log!("i: {}, e: {}", i, e);
        
        if i == last_el {
            temp.insert(e.to_string(), json!(value));
            // near_sdk::log!("final insert {:?}", temp);
            break;
        }
        
        // near_sdk::log!("Setting temp");
        if let Some(v) = temp.get_mut(e).and_then(|v| v.as_object_mut()) {
            temp = v
        } else {
            return false
        }
        // near_sdk::log!("temp outer {:?}", temp);
    }    

    true
}

pub(crate) fn handle_fc_args(
    output_args: &String, 
    user_args: &Option<Vec<Option<String>>>, 
    user_args_rule: &Option<UserArgsRule>,
    account_id_field: &Option<String>,
    account_id: &AccountId,
    drop_id_field: &Option<String>,
    drop_id: &DropId,
    key_id_field: &Option<String>,
    key_id: &u64,
    funder_id_field: &Option<String>,
    funder_id: &AccountId,
    i: usize,
) -> Result<String, String> {
    let try_json: Result<Value, _> = from_str(&output_args);
    if try_json.is_err() {
        return Err("Cannot cast args to JSON. Returning and decrementing keys".to_string());
    }

    let mut output_args_json = try_json.unwrap();

    // Check if user provided args is present and fc_args is some
    if let (Some(rule), Some(user_args_str)) = (user_args_rule, user_args.as_ref().and_then(|a| a[i].clone())) {
        let try_user_json: Result<Value, _> = from_str(&user_args_str);
        if try_user_json.is_ok() {
            let mut user_args_json = try_user_json.unwrap();
            
            match rule {
                UserArgsRule::AllUser => {
                    output_args_json = user_args_json;
                }
                UserArgsRule::FunderPreferred => {
                    // Take the final args string and merge the user args into it and overwrite any duplicate keys
                    merge_json(&mut user_args_json, &output_args_json);
                    output_args_json = user_args_json;
                }
                UserArgsRule::UserPreferred => {
                    // Take the final args string and merge the user args into it and overwrite any duplicate keys
                    merge_json(&mut output_args_json, &user_args_json);
                }
            }
        } else {
            near_sdk::log!(
                "Cannot cast user provided args to JSON. Disregarding user args and continuing"
            );
        }
    }

    output_args_json["keypom_args"] = json!({
        "account_id_field": account_id_field,
        "drop_id_field": drop_id_field,
        "key_id_field": key_id_field,
        "funder_id_field": funder_id_field,
    });

    // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
    if let Some(field) = account_id_field.as_ref() {
        if add_keypom_field(&mut output_args_json, field, account_id.to_string()) == false {
            return Err("Cannot add account ID to specified field. Returning and decrementing keys".to_string());
        };

        near_sdk::log!(
            "Adding claimed account ID: {:?} to specified field: {:?}",
            account_id,
            field
        );
    }

    // Add drop_id
    if let Some(field) = drop_id_field.as_ref() {
        if add_keypom_field(&mut output_args_json, field, drop_id.to_string()) == false {
            return Err("Cannot add drop ID to specified field. Returning and decrementing keys".to_string());
        }
        near_sdk::log!("Adding drop ID: {:?} to specified field {:?}", drop_id, field);
    }

    // Add the key_id
    if let Some(field) = key_id_field.as_ref() {
        if add_keypom_field(&mut output_args_json, field, key_id.to_string()) == false {
            return Err("Cannot add key ID to specified field. Returning and decrementing keys".to_string());
        }
        near_sdk::log!("Adding key ID: {:?} to specified field {:?}", key_id, field);
    }

    // Add the funder_id
    if let Some(field) = funder_id_field.as_ref() {
        if add_keypom_field(&mut output_args_json, field, funder_id.to_string()) == false {
            return Err("Cannot add funder ID to specified field. Returning and decrementing keys".to_string());
        }
        near_sdk::log!("Adding funder ID: {:?} to specified field {:?}", funder_id, field);
    }

    if to_string(&output_args_json).is_err() {
        return Err("Cannot cast final args to JSON. Returning and decrementing keys".to_string());
    }

    return Ok(to_string(&output_args_json).unwrap());
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
            let mut output_args = method.args.clone();
            if output_args.len() == 0 {
                output_args = "{}".to_string();
            }

            match handle_fc_args(
                &mut output_args, 
                &fc_args, 
                &method.user_args_rule, 
                &method.account_id_field, 
                &account_id, 
                &method.drop_id_field, 
                &drop_id, 
                &method.key_id_field, 
                &key_id, 
                &method.funder_id_field, 
                &funder_id, 
                i
            ) {
                Ok(args) => {
                    output_args = args;
                }
                Err(e) => {
                    near_sdk::log!("Error handling FC args: {:?}", e);
                    return;
                }
            }

            // Call function with the min GAS and attached_deposit. all unspent GAS will be added on top
            Promise::new(method.receiver_id.clone()).function_call_weight(
                method.method_name.clone(),
                output_args.as_bytes().to_vec(),
                // The claim is successful so attach the amount to refund to the attached_deposit instead of refunding the funder.
                method.attached_deposit.0,
                gas,
                GasWeight(1),
            );
        }
    }
}
