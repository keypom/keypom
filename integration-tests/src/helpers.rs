use crate::*;
use models::*;
use near_workspaces::types::{KeyType, SecretKey};
use near_workspaces::{AccessKey, Account, Contract};

pub fn sign_kp_message(
    sk: &near_crypto::SecretKey,
    nonce: u32,
    message: &String,
) -> near_crypto::Signature {
    println!("sk: {:?}, Signing message: {}", sk, message);
    sk.sign(&format!("{}{}", message, nonce).as_bytes())
}

pub fn verify_kp_signature(
    pk: &near_crypto::PublicKey,
    sig: &near_crypto::Signature,
    nonce: u32,
    message: &String,
) {
    let msg = format!("{}{}", message, nonce);
    let is_valid = sig.verify(msg.as_bytes(), pk);
    println!("Is Valid: {}", is_valid);
}

pub async fn get_sig_meta(contract: Contract) -> Result<SignatureMeta, anyhow::Error> {
    let global_sk = contract
        .view("get_global_secret_key")
        .await?
        .json::<String>()?;
    let message = contract
        .view("get_global_secret_key")
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
