use near_sdk::{
    env::{
        promise_batch_create,
        promise_batch_then,
        promise_batch_action_function_call_weight,
    },
    GasWeight
};

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

pub(crate) fn set_user_markers(
    output_args: &mut String,
    user_arg: &String
) -> Result<String, String> {
    near_sdk::log!("Setting user markers");

    let try_json: Result<Value, _> = from_str(&user_arg);
    if try_json.is_err() {
        return Err("Cannot cast args to JSON. Returning and decrementing keys".to_string());
    }
    
    if let Some(obj) = try_json.unwrap().as_object() {
        near_sdk::log!("User marker obj: {:?}", obj);

        for (key, value) in obj {
            if value.is_string() {
                let mut val_to_insert = value.to_string();
                rem_first_and_last(&mut val_to_insert);
                let key = format!(":\"{}\"", key.to_uppercase());
                val_to_insert = format!(":\"{}\"", val_to_insert);
                
                near_sdk::log!("(STRING) replacing {:?} with {:?}", key.to_string(), val_to_insert);
                *output_args = output_args.replace(&key.to_string(), &val_to_insert);
            } else if value.is_object() {
                let mut val_to_insert = to_string(&value).unwrap();
                let key = format!(":\"{}\"", key.to_uppercase());
                val_to_insert = format!(":{}", val_to_insert);
                
                near_sdk::log!("(OBJECT) replacing {:?} with {:?}", key.to_string(), val_to_insert);
                *output_args = output_args.replace(&key.to_string(), &val_to_insert);
            }
        }
    }

    Ok("success".to_string())
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
        if temp.contains_key(e) {
            if let Some(v) = temp.get_mut(e).unwrap().as_object_mut() {
                temp = v; 
            } else {
                return false;
            }
        } else {
            temp.insert(e.clone().to_string(), json!({})); 
            temp = temp.get_mut(e).unwrap().as_object_mut().unwrap();
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
    if output_args.len() > 4096 {
        near_sdk::log!("Args length is greater than 4096. Skipping JSON steps.");
        
        let keypom_args = KeypomArgs {
            account_id_field: account_id_field.clone(),
            drop_id_field: drop_id_field.clone(),
            key_id_field: key_id_field.clone(),
            funder_id_field: funder_id_field.clone(),
        };
        
        if output_args.contains("\"keypom_args\"") {
            return Err("Keypom Args detected in client args. Returning and decrementing keys".to_string());
        }

        let mut modified_args = output_args.clone();

        // Check if user provided args is present and fc_args is some
        if let (Some(rule), Some(user_args_str)) = (user_args_rule, user_args.as_ref().and_then(|a| a[i].clone())) {
            match rule {
                UserArgsRule::UserPreferred => {
                    let res = set_user_markers(&mut modified_args, &user_args_str);
                    if res.is_err() {
                        return Err(res.err().unwrap());
                    }
                }
                _ => {
                    near_sdk::log!("User args rule is not UserPreferred. Skipping marker logic");
                }
            }
        }

        modified_args.insert_str(
            modified_args.len() - 1,
            &format!(
                ",\"keypom_args\":{}",
                near_sdk::serde_json::to_string(&keypom_args).unwrap()
            ),
        );

        // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
        if let Some(field) = account_id_field.as_ref() {
            if modified_args.contains(field) {
                return Err("Account ID Field value present in args already. Returning and decrementing keys.".to_string());
            }

            modified_args.insert_str(
                modified_args.len() - 1,
                &format!(",\"{}\":\"{}\"", field, account_id),
            );
            near_sdk::log!(
                "Adding claimed account ID: {:?} to specified field: {:?}",
                account_id,
                field
            );
        }

        // Add drop_id
        if let Some(field) = drop_id_field.as_ref() {
            if modified_args.contains(field) {
                return Err("Drop ID Field value present in args already. Returning and decrementing keys.".to_string());
            }

            modified_args.insert_str(
                modified_args.len() - 1,
                &format!(",\"{}\":\"{}\"", field, drop_id),
            );
            near_sdk::log!(
                "Adding drop ID: {:?} to specified field: {:?}",
                drop_id,
                field
            );
        }

        // Add the key_id
        if let Some(field) = key_id_field.as_ref() {
            if modified_args.contains(field) {
                return Err("Key ID Field value present in args already. Returning and decrementing keys.".to_string());
            }

            modified_args.insert_str(
                modified_args.len() - 1,
                &format!(",\"{}\":\"{}\"", field, key_id),
            );
            near_sdk::log!(
                "Adding key ID: {:?} to specified field: {:?}",
                key_id,
                field
            );
        }

        // Add the funder_id
        if let Some(field) = funder_id_field.as_ref() {
            if modified_args.contains(field) {
                return Err("Funder ID Field value present in args already. Returning and decrementing keys.".to_string());
            }

            modified_args.insert_str(
                modified_args.len() - 1,
                &format!(",\"{}\":\"{}\"", field, funder_id),
            );
            near_sdk::log!(
                "Adding funder account ID: {:?} to specified field: {:?}",
                funder_id,
                field
            );
        }

        return Ok(modified_args.to_string());
    } else {
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
                return Err("Cannot cast user provided args to JSON. Returning and decrementing keys".to_string());
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
}

#[near_bindgen]
impl Keypom {
    // Internal method_name for transfer NFTs.
    pub(crate) fn internal_fc_execute(
        &mut self,
        methods: &Vec<MethodData>,
        key_id: u64,
        account_id: AccountId,
        funder_id: AccountId,
        drop_id: DropId,
        fc_args: Option<Vec<Option<String>>>,
    ) {
        /*
            Function Calls
        */
        let mut promises: Vec<u64> = vec![];

        for (i, method) in methods.iter().enumerate() {
            let mut receiver_id = method.receiver_id.to_string();
            if receiver_id == env::current_account_id().to_string() {
                near_sdk::log!("Cannot set account_id to current account ID. Returning and decrementing keys");
                return;
            }

            if method.receiver_to_claimer.unwrap_or(false) {
                receiver_id = account_id.to_string();
            }
            
            near_sdk::log!("(TOP of for loop): initial receiver ID: {:?} for method: {:?}", receiver_id, method.method_name);
            
            let gas = method.attached_gas.unwrap_or(Gas(0));
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

            let actual_receiver: Result<AccountId, _> = receiver_id.parse();

            if actual_receiver.is_err() {
                near_sdk::log!("Error converting custom receiver ID to type account ID: {:?}", actual_receiver.err());
                return;
            }

            // start new promise batch or chain with previous promise batch
            let id = if promises.len() == 0 {
                promise_batch_create(&actual_receiver.unwrap())
            } else {
                promise_batch_then(
                    promises[promises.len() - 1],
                    &actual_receiver.unwrap(),
                )
            };
            promises.push(id);

            promise_batch_action_function_call_weight(
                id,
                &method.method_name,
                output_args.as_bytes(),
                method.attached_deposit.0,
                gas,
                GasWeight(1),
            )
        }
    }
}
