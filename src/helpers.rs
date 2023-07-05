use crate::*;

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_drop_id(drop_id: &String) -> CryptoHash {
    env::sha256_array(drop_id.as_bytes())
}

/// Helper function to convert yoctoNEAR to $NEAR with 7 decimals of precision.
pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
    //10^17 yoctoNEAR (1 NEAR would be 10_000_000). This is to give a precision of 7 decimal places.
    let formatted_near = yocto / 100_000_000_000_000_000;
    let near = formatted_near as f64 / 10_000_000f64;

    near
}

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn asset_id_from_ext_asset(ext_asset: &ExtAsset) -> AssetId {
    match ext_asset {
        ExtAsset::FTAsset(ft_data) => ft_data.contract_id.to_string(),
        _ => env::panic_str("Asset type not supported")
    }
}

/// Checks that the attached deposit is greater than the required deposit and refunds any excess
pub(crate) fn internal_refund_excess_deposit(required_deposit: Balance) {
    let attached_deposit = env::attached_deposit();
    require!(attached_deposit >= required_deposit, format!("Attached deposit {} must be greater than required deposit {}", attached_deposit, required_deposit));

    if attached_deposit > required_deposit {
        let refund_amount = attached_deposit - required_deposit;
        near_sdk::log!("Refunding {} excess deposit", refund_amount);
        Promise::new(env::predecessor_account_id()).transfer(refund_amount);
    }
}

/// Query for the key's current use number given the drop and key info
pub(crate) fn get_key_cur_use(drop: &InternalDrop, key_info: &InternalKeyInfo) -> UseNumber {
    drop.uses_per_key - key_info.remaining_uses + 1
}

/// Used to calculate the base allowance needed given attached GAS
pub(crate) fn calculate_base_allowance(yocto_per_gas: Balance, attached_gas: Gas, should_log: bool) -> Balance {
    let prepaid: u64 = attached_gas.0 + GAS_PER_CCC.0;

    // Get the number of CCCs you can make with the attached GAS
    // 5 TGas GAS_PER_CCC
    let calls_with_gas = (prepaid / GAS_PER_CCC.0) as f32;
    // Get the constant used to pessimistically calculate the required allowance
    let pow_outcome = 1.032_f32.powf(calls_with_gas);

    // Get the required GAS based on the calculated constant
    // 2.5 TGas receipt cost
    let required_allowance = ((prepaid + RECEIPT_GAS_COST.0) as f32 * pow_outcome
        + RECEIPT_GAS_COST.0 as f32) as Balance
        * yocto_per_gas;

    if should_log {
        near_sdk::log!(
            "{} calls with {} attached GAS. Pow outcome: {}. Required Allowance: {}",
            calls_with_gas,
            prepaid,
            pow_outcome,
            required_allowance
        );
    }

    required_allowance
}