use crate::*;
use models::*;
use near_sdk::AccountId;
use near_workspaces::result::ExecutionFinalResult;
use near_workspaces::types::{KeyType, SecretKey};
use near_workspaces::{AccessKey, Account, Contract};

pub async fn call_contract(
    account: &Account,
    contract: &AccountId,
    method: &str,
    args: Option<serde_json::Value>,
    deposit: Option<NearToken>,
) -> ExecutionFinalResult {
    account
        .call(contract, method)
        .args_json(args.unwrap_or(serde_json::Value::Null))
        .deposit(deposit.unwrap_or(NearToken::from_yoctonear(0)))
        .transact()
        .await
        .unwrap()
}

pub fn sign_kp_message(sk: &near_crypto::SecretKey, nonce: u32, message: &String) -> Base64VecU8 {
    let signature = match sk.sign(&format!("{}{}", message, nonce).as_bytes()) {
        near_crypto::Signature::ED25519(sig) => sig,
        _ => panic!("Invalid signature type"),
    };

    Base64VecU8(signature.to_bytes().to_vec())
}

pub fn verify_kp_signature(
    pk: &near_crypto::PublicKey,
    sig: &near_crypto::Signature,
    nonce: u32,
    message: &String,
) -> bool {
    let msg = format!("{}{}", message, nonce);
    sig.verify(msg.as_bytes(), pk)
}

pub async fn get_sig_meta(contract: Contract) -> Result<SignatureMeta, anyhow::Error> {
    let global_sk = contract
        .view("get_global_secret_key")
        .await?
        .json::<String>()?;
    let message = contract
        .view("get_signing_message")
        .await?
        .json::<String>()?;

    let sk: SecretKey = global_sk.parse().unwrap();
    Ok(SignatureMeta {
        message,
        secret_key: sk,
    })
}

pub async fn get_drop_info(contract: Contract, drop_id: String) -> Result<ExtDrop, anyhow::Error> {
    let drop_info = contract
        .view("get_drop_information")
        .args_json(json!({"drop_id": drop_id}))
        .await?
        .json::<ExtDrop>()?;

    Ok(drop_info)
}

pub fn generate_keypairs(num_keys: u16) -> Vec<SecretKey> {
    let mut sks = vec![];

    for _ in 0..num_keys {
        // Generate a keypair
        sks.push(SecretKey::from_random(KeyType::ED25519));
    }

    sks
}
