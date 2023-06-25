use crate::*;

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_drop_id(drop_id: &String) -> CryptoHash {
    env::sha256_array(drop_id.as_bytes())
}

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn asset_id_from_ext_asset(ext_asset: &ExtAsset) -> AssetId {
    match ext_asset {
        ExtAsset::FTAsset(ft_data) => ft_data.contract_id.to_string(),
        _ => env::panic_str("Asset type not supported")
    }
}

/// Checks that the attached deposit is greater than the required deposit and refunds any excess
pub(crate) fn internal_refund_excess_deposit(required_deposit: u128) {
    let attached_deposit = env::attached_deposit();
    require!(attached_deposit >= required_deposit, format!("Attached deposit {} must be greater than required deposit {}", attached_deposit, required_deposit));

    if attached_deposit > required_deposit {
        let refund_amount = attached_deposit - required_deposit;
        Promise::new(env::predecessor_account_id()).transfer(refund_amount);
    }
}

/// Loops through public keys and adds them to the current contract account
pub(crate) fn internal_add_keys_to_account(public_keys: &Vec<PublicKey>, method_names: &str, allowance: u128) {
    let current_account_id = &env::current_account_id();

    // Create a new promise batch to create all the access keys
    let promise = env::promise_batch_create(current_account_id);

    // Loop through each public key and create the access keys
    for pk in public_keys {
        // Must assert in the loop so no access keys are made?
        env::promise_batch_action_add_key_with_function_call(
            promise,
            pk,
            0,
            allowance,
            current_account_id,
            method_names,
        );
    }

    env::promise_return(promise);
}

/// Used to calculate the base allowance needed given attached GAS
pub(crate) fn calculate_base_allowance(yocto_per_gas: u128, attached_gas: Gas) -> u128 {
    let prepaid: u64 = attached_gas.0 + GAS_PER_CCC.0;

    // Get the number of CCCs you can make with the attached GAS
    // 5 TGas GAS_PER_CCC
    let calls_with_gas = (prepaid / GAS_PER_CCC.0) as f32;
    // Get the constant used to pessimistically calculate the required allowance
    let pow_outcome = 1.03_f32.powf(calls_with_gas);

    // Get the required GAS based on the calculated constant
    // 2.5 TGas receipt cost
    let required_allowance = ((prepaid + RECEIPT_GAS_COST.0) as f32 * pow_outcome
        + RECEIPT_GAS_COST.0 as f32) as u128
        * yocto_per_gas;
    near_sdk::log!(
        "{} calls with {} attached GAS. Pow outcome: {}. Required Allowance: {}",
        calls_with_gas,
        prepaid,
        pow_outcome,
        required_allowance
    );

    required_allowance
}