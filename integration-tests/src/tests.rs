use std::str::FromStr;

use helpers::*;
use near_crypto::{KeyType, SecretKey, Signer};
use near_gas::NearGas;
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
    claims_testing(&alice, keypom_contract).await?;
    Ok(())
}

/// Test whether or not the global key works
async fn claims_testing(user: &Account, keypom_contract: Contract) -> anyhow::Result<()> {
    let global_key_info = get_sig_meta(keypom_contract.clone()).await?;
    let mut kp_account = keypom_contract.as_account().clone();

    let keys = generate_keypairs(1);
    let sk = keys[0].clone();
    // Create a 1 $NEAR linkdrop
    let args = json!(
    {
        "drop_id": "my_drop",
        "key_data": [{
            "public_key": sk.public_key(),
        }],
        "asset_data": [{
            "uses": 2,
            "assets": [{
                "yoctonear": NearToken::from_near(1).as_yoctonear().to_string()
            }]
        }]
    });
    let res = user
        .call(keypom_contract.id(), "create_drop")
        .args_json(args)
        .deposit(NearToken::from_near(3))
        .transact()
        .await?;
    assert!(res.is_success());

    let signing_key: near_crypto::SecretKey = sk.to_string().parse().unwrap();
    let sig_0 = sign_kp_message(&signing_key, 0, &global_key_info.message);
    let sig_1 = sign_kp_message(&signing_key, 1, &global_key_info.message);

    // Try to claim the drop with the wrong global secret key
    let mut signature_result = kp_account
        .call(keypom_contract.id(), "claim")
        .args_json(
            json!({"account_id": user.id(), "signature": sig_0, "linkdrop_pk": sk.public_key()}),
        )
        .transact()
        .await?
        .into_result();
    let mut error = signature_result.expect_err("Error expected: Wrong global secret key");
    assert!(format!("{error:?}").contains("Only Contract Key Can Call This Method"));

    // Set the global secret key but this time, sign the wrong message with a different public key
    kp_account.set_secret_key(global_key_info.secret_key);
    signature_result = kp_account
        .call(keypom_contract.id(), "claim")
        .args_json(
            json!({"account_id": user.id(), "signature": sig_1, "linkdrop_pk": sk.public_key()}),
        )
        .transact()
        .await?
        .into_result();
    error = signature_result.expect_err("Err Expected: wrong signature ");
    assert!(format!("{error:?}").contains("Invalid signature for public key"));

    // Now use the correct signature
    let key_info = get_key_info(&keypom_contract, sk.public_key(), true)
        .await?
        .unwrap();
    let mut execution_res = kp_account
        .call(keypom_contract.id(), "claim")
        .args_json(
            json!({"account_id": user.id(), "signature": sig_0, "linkdrop_pk": sk.public_key()}),
        )
        .gas(NearGas::from_gas(
            key_info.required_gas.parse::<u64>().unwrap(),
        ))
        .transact()
        .await?;
    let mut execution_val = execution_res.json::<bool>()?;
    assert!(execution_val);
    get_key_info(&keypom_contract, sk.public_key(), true).await?;

    // Use the correct signature but with an incremented nonce
    execution_res = kp_account
        .call(keypom_contract.id(), "claim")
        .args_json(
            json!({"account_id": user.id(), "signature": sig_1, "linkdrop_pk": sk.public_key()}),
        )
        .gas(NearGas::from_gas(
            key_info.required_gas.parse::<u64>().unwrap(),
        ))
        .transact()
        .await?;

    execution_val = execution_res.json::<bool>()?;
    assert!(execution_val);

    get_key_info(&keypom_contract, sk.public_key(), false).await?;

    println!("      Passed ✅ test_simple_approve");
    Ok(())
}
