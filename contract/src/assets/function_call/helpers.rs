use serde_json::{Value, from_str, to_string};
use crate::*;
/// b will overwrite a and `a` will be mutated
pub fn merge_json(a: &mut Value, b: &Value) {
    match (a, b) {
        (&mut Value::Object(ref mut a), &Value::Object(ref b)) => {
            for (k, v) in b {
                merge_json(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

/// Removes the first and last character from a string
pub(crate) fn rem_first_and_last_char(value: &mut String) {
    value.pop();
    value.remove(0);
}

/// Given the actual args to the function call, and some user defined arguments
/// Set / replace the markers with what the user passed in
pub(crate) fn set_user_markers(
    output_args: &mut String,
    user_arg: &String
) -> Result<(), String> {
    near_sdk::log!("Setting user markers");

    let try_json: Result<Value, _> = from_str(&user_arg);
    if try_json.is_err() {
        return Err("Cannot cast args to JSON. Skipping method.".to_string());
    }
    
    if let Some(obj) = try_json.unwrap().as_object() {
        near_sdk::log!("User marker obj: {:?}", obj);

        for (key, value) in obj {
            if value.is_string() {
                let mut val_to_insert = value.to_string();
                rem_first_and_last_char(&mut val_to_insert);
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

    Ok(())
}

pub(crate) fn insert_keypom_arg(
    output_args: &mut String,
    optional_field: &Option<String>,
    value: String
) -> Result<(), String> {
    // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
    if let Some(field) = optional_field {
        if output_args.contains(field) {
            return Err(format!("Keypom field {} present in args already. Skipping method.", field));
        }

        output_args.insert_str(
            output_args.len() - 1,
            &format!(",\"{}\":\"{}\"", field, value),
        );
        near_sdk::log!(
            "Added Value {} into {} For Keypom Args.",
            value,
            field
        );
    }

    Ok(())
}

pub(crate) fn handle_fc_args(
    output_args: &mut String, 
    keypom_args: Option<KeypomInjectedArgs>,
    user_args_rule: &Option<UserArgsRule>,
    user_args: &Option<Vec<Option<String>>>, 
    account_id: &AccountId,
    drop_id: &DropId,
    key_id: &String,
    funder_id: &AccountId,
    i: usize,
) -> Result<(), String> {
    // Add keypom args and set any user markers
    let keypom_args = keypom_args.unwrap_or(KeypomInjectedArgs { 
        account_id_field: None,
        drop_id_field: None,
        key_id_field: None,
        funder_id_field: None
    });
    
    if output_args.contains("\"keypom_args\"") {
        return Err("Keypom Args detected in client args. Returning and decrementing keys".to_string());
    }

    // Check if user provided args is present and fc_args is some
    if let (Some(rule), Some(user_args_str)) = (user_args_rule, user_args.as_ref().and_then(|a| a[i].clone())) {
        match rule {
            UserArgsRule::UserPreferred => {
                set_user_markers(output_args, &user_args_str)?;
            }
            _ => {
                near_sdk::log!("User args rule is not UserPreferred. Skipping marker logic");
            }
        }
    }

    output_args.insert_str(
        output_args.len() - 1,
        &format!(
            ",\"keypom_args\":{}",
            near_sdk::serde_json::to_string(&keypom_args).unwrap()
        ),
    );

    insert_keypom_arg(
        output_args,
        &keypom_args.account_id_field,
        account_id.to_string()
    )?;
    insert_keypom_arg(
        output_args,
        &keypom_args.drop_id_field,
        drop_id.to_string()
    )?;
    insert_keypom_arg(
        output_args,
        &keypom_args.key_id_field,
        key_id.to_string()
    )?;
    insert_keypom_arg(
        output_args,
        &keypom_args.funder_id_field,
        funder_id.to_string()
    )?;

    if output_args.len() <= 4096 {
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

        if to_string(&output_args_json).is_err() {
            return Err("Cannot cast final args to JSON. Returning and decrementing keys".to_string());
        }

        *output_args = to_string(&output_args_json).unwrap();
    }

    return Ok(());
}