use std::str::FromStr;

use helpers::*;
use near_crypto::{KeyType, SecretKey, Signer};
use near_sdk::{json_types::Base64VecU8, NearToken};
use near_units::parse_near;
use near_workspaces::{AccessKey, Account, Contract};
use serde_json::json;

mod helpers;
mod models;

const KEYPOM_WASM_PATH: &str = "./out/keypom.wasm";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initiate environemnt
    let worker = near_workspaces::sandbox().await?;

    // deploy contracts
    let keypom_wasm = match std::fs::read(KEYPOM_WASM_PATH) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading wasm file: {}", err);
            // Handle the error appropriately, e.g., return an error or panic with a more informative message.
            std::process::exit(1);
        }
    };
    let keypom_contract = worker.dev_deploy(&keypom_wasm).await?;

    // create accounts
    let owner = worker.root_account().unwrap();
    let alice = owner
        .create_subaccount("alice")
        .initial_balance(NearToken::from_near(30))
        .transact()
        .await?
        .into_result()?;

    let keys = generate_keypairs(1);

    // Initialize contracts
    keypom_contract
        .call("new")
        .args_json(json!({
            "root_account": owner.id(),
            "owner_id": owner.id(),
            "signing_pk": keys[0].public_key(),
            "signing_sk": keys[0],
            "message": "keypom_is_lit"
        }))
        .transact()
        .await?;

    // begin tests
    test_global_key(&alice, keypom_contract).await?;
    Ok(())
}

async fn test_signatures(
    owner: &Account,
    user: &Account,
    keypom_contract: Contract,
) -> anyhow::Result<()> {
    let global_key_info = get_sig_meta(keypom_contract.clone()).await?;
    let mut kp_account = keypom_contract.as_account().clone();

    let keys = generate_keypairs(1);
    let mut args = json!({"drop_id": "my_drop", "key_data": [{
        "public_key": keys[0].public_key(),
    }], "asset_data": [{"uses": 1, "assets": []}]
    });
    let res = call_contract(
        user,
        keypom_contract.id(),
        "create_drop",
        Some(args),
        Some(NearToken::from_near(1)),
    )
    .await;

    println!("Create Drop Res: {:?}", res);

    let kp_keys = keypom_contract.as_account().view_access_keys().await?;
    println!("Keys: {:?}", kp_keys);
    assert!(kp_keys.len() == 2);

    // Try to call the contract with the wrong secret key
    args = json!({"signature": owner.id(), "pk": keys[0].public_key()});
    let mut signature_result =
        call_contract(&kp_account, keypom_contract.id(), "test", Some(args), None).await;
    assert!(!signature_result.is_success());

    let drop_info = get_drop_info(keypom_contract.clone(), "my_drop".to_string()).await?;
    println!("Drop Info: {:?}", drop_info);

    // Set the global secret key and try again
    kp_account.set_secret_key(global_key_info.secret_key);
    args = json!({"signature": owner.id(), "pk": keys[0].public_key()});
    signature_result =
        call_contract(&kp_account, keypom_contract.id(), "test", Some(args), None).await;
    assert!(signature_result.is_success());

    println!("      Passed ✅ test_simple_approve");
    Ok(())
}

/// Test whether or not the global key works
async fn test_global_key(user: &Account, keypom_contract: Contract) -> anyhow::Result<()> {
    let global_key_info = get_sig_meta(keypom_contract.clone()).await?;
    let mut kp_account = keypom_contract.as_account().clone();

    let keys = generate_keypairs(1);
    let sk = keys[0].clone();
    user.call(keypom_contract.id(), "create_drop")
        .args_json(json!({"drop_id": "my_drop", "key_data": [{
            "public_key": sk.public_key(),
        }], "asset_data": [{"uses": 1, "assets": []}]
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;

    let signing_key: near_crypto::SecretKey = sk.to_string().parse().unwrap();
    let sig_0 = sign_kp_message(&signing_key, 0, &global_key_info.message);
    let sig_1 = sign_kp_message(&signing_key, 1, &global_key_info.message);

    // Try to call the contract with the wrong global secret key
    let mut signature_result = kp_account
        .call(keypom_contract.id(), "verify_signature")
        .args_json(json!({"signature": sig_0, "pk": sk.public_key()}))
        .transact()
        .await?
        .into_result();
    let mut error = signature_result.expect_err("Wrong global secret key");
    assert!(format!("{error:?}").contains("Only Contract Key Can Call This Method"));

    // Set the global secret key but this time, sign the wrong message with a different public key
    kp_account.set_secret_key(global_key_info.secret_key);
    let mut success_value = kp_account
        .call(keypom_contract.id(), "verify_signature")
        .args_json(json!({"signature": sig_1, "pk": sk.public_key()}))
        .transact()
        .await?
        .json::<bool>()
        .unwrap();

    assert!(!success_value);

    // Now use the correct signature
    success_value = kp_account
        .call(keypom_contract.id(), "verify_signature")
        .args_json(json!({"signature": sig_0, "pk": sk.public_key()}))
        .transact()
        .await?
        .json::<bool>()
        .unwrap();

    assert!(success_value);
    // Use the correct signature but with an incremented nonce
    success_value = kp_account
        .call(keypom_contract.id(), "verify_signature")
        .args_json(json!({"signature": sig_1, "pk": sk.public_key()}))
        .transact()
        .await?
        .json::<bool>()?;
    assert!(success_value);

    println!("      Passed ✅ test_simple_approve");
    Ok(())
}
