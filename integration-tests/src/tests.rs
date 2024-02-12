use std::str::FromStr;

use helpers::*;
use near_crypto::{KeyType, SecretKey, Signer};
use near_sdk::NearToken;
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
    test_global_key(&owner, &alice, keypom_contract).await?;
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
    println!("Keys: {:?}", keys);
    let res = user
        .call(keypom_contract.id(), "create_drop")
        .args_json(json!({"drop_id": "my_drop", "key_data": [{
            "public_key": keys[0].public_key(),
        }], "asset_data": [{"uses": 1, "assets": []}]
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;

    println!("Create Drop Res: {:?}", res);

    let kp_keys = keypom_contract.as_account().view_access_keys().await?;
    println!("Keys: {:?}", kp_keys);
    assert!(kp_keys.len() == 2);

    // Try to call the contract with the wrong secret key
    let mut signature_result = kp_account
        .call(keypom_contract.id(), "test")
        .args_json(json!({"signature": owner.id(), "pk": keys[0].public_key()}))
        .transact()
        .await?;
    assert!(!signature_result.is_success());

    let drop_info = get_drop_info(keypom_contract.clone(), "my_drop".to_string()).await?;
    println!("Drop Info: {:?}", drop_info);

    // Set the global secret key and try again
    kp_account.set_secret_key(global_key_info.secret_key);
    signature_result = kp_account
        .call(keypom_contract.id(), "test")
        .args_json(json!({"signature": owner.id(), "pk": keys[0].public_key()}))
        .transact()
        .await?;
    println!("Signature: {:?}", signature_result);
    assert!(signature_result.is_success());

    let message = "keypom_is_lit".to_string();
    let pub_key = "ed25519:4WvV8m9bKg7a6b6XnQ5u7L2yQ4nV9P4Uq1G6WZtM5X6k";
    let sk = "ed25519:v9j3b9F1Yh5KZr7Fj8y8Tb1m3R9h5Qh4R3j7v5Fj8y8Tb1m3R9h5Qh4R3j7v5Fj8y8Tb1m3R9h5Qh4R3j7v5Fj8y8Tb1m3R9h5Qh4R3j7v5Fj8y8Tb1m3R9h5Qh4R3j7v5Fj8y8Tb1m3R9h5Qh4R3j7v5Fj8y8Tb1m3R9h5Qh4R3j7v5Fj8y8Tb1m";

    println!("      Passed ✅ test_simple_approve");
    Ok(())
}

async fn test_global_key(
    owner: &Account,
    user: &Account,
    keypom_contract: Contract,
) -> anyhow::Result<()> {
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
    let signature = sign_kp_message(&signing_key, 0, &global_key_info.message);
    verify_kp_signature(
        &signing_key.public_key(),
        &signature,
        0,
        &global_key_info.message,
    );

    // Try to call the contract with the wrong secret key
    let mut signature_result = kp_account
        .call(keypom_contract.id(), "verify_signature")
        .args_json(json!({"signature": signature, "pk": sk.public_key()}))
        .transact()
        .await?;
    assert!(!signature_result.is_success());

    let drop_info = get_drop_info(keypom_contract.clone(), "my_drop".to_string()).await?;
    println!("Drop Info: {:?}", drop_info);

    // Set the global secret key and try again
    kp_account.set_secret_key(global_key_info.secret_key);
    signature_result = kp_account
        .call(keypom_contract.id(), "verify_signature")
        .args_json(json!({"signature": signature, "pk": sk.public_key()}))
        .transact()
        .await?;
    println!("Signature: {:?}", signature_result);
    assert!(signature_result.is_success());

    println!("      Passed ✅ test_simple_approve");
    Ok(())
}
