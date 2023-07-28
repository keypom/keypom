use crate::*;

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_string(string: &String) -> CryptoHash {
    env::sha256_array(string.as_bytes())
}

/// Helper function to convert yoctoNEAR to $NEAR with 7 decimals of precision.
pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
    //10^17 yoctoNEAR (1 NEAR would be 10_000_000). This is to give a precision of 7 decimal places.
    let formatted_near = yocto / 100_000_000_000_000_000;
    let near = formatted_near as f64 / 10_000_000f64;

    near
}

/// Query for the key's current use number given the drop and key info
pub(crate) fn get_key_cur_use(drop: &InternalDrop, key_info: &InternalKeyInfo) -> UseNumber {
    drop.max_key_uses - key_info.remaining_uses + 1
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

/// Helper function that returns the total cost for a given key as well as its allowance
/// This key can be partially used or not
pub(crate) fn get_total_costs_for_key(
    total_cost_for_keys: &mut Balance,
    total_allowance_for_keys: &mut Balance,
    remaining_uses: UseNumber, 
    max_uses_per_key: UseNumber, 
    asset_by_id: &UnorderedMap<AssetId, InternalAsset>,
    key_use_behaviors: &InternalKeyUseBehaviors,
    drop_config: &Option<DropConfig>
) {
    // For every remaining use, we need to loop through all assets and refund
    for cur_use in 1..=remaining_uses {
        let use_to_refund = max_uses_per_key - remaining_uses + cur_use;

        // Get the total costs for this current use
        get_total_costs_for_use(
            total_cost_for_keys,
            total_allowance_for_keys,
            use_to_refund,
            asset_by_id,
            key_use_behaviors,
            drop_config
        );
    }
}

/// Helper function to get the internal key behavior for a given use number
pub(crate) fn get_internal_key_behavior_for_use (
    key_use_behaviors: &InternalKeyUseBehaviors,
    use_number: &UseNumber
) -> InternalKeyBehaviorForUse {
    // Get the assets metadata for this use number
    match key_use_behaviors {
        InternalKeyUseBehaviors::AllUses(data) => InternalKeyBehaviorForUse {
            assets_metadata: data.assets_metadata.clone(),
            config: None
        },
        // The use number should be decremented by 1 since the vector is zero indexed
        InternalKeyUseBehaviors::PerUse(data) => data.get((use_number - 1) as usize).expect("Use number not found").clone()
    }
}

pub(crate) fn get_total_costs_for_use(
    total_cost_for_use: &mut Balance,
    total_allowance_for_use: &mut Balance,
    use_number: UseNumber,
    asset_by_id: &UnorderedMap<AssetId, InternalAsset>,
    key_use_behaviors: &InternalKeyUseBehaviors,
    drop_config: &Option<DropConfig>
) {
    let InternalKeyBehaviorForUse { config: use_config, assets_metadata } = get_internal_key_behavior_for_use(key_use_behaviors, &use_number);
    
    let usage_config = use_config.as_ref().and_then(|c| c.get_usage_config()).or(drop_config.as_ref().and_then(|c| c.get_usage_config()));        
    // If the config usage's permission field is set to Claim, the base should be set accordingly. In all other cases, it should be the base for CAAC
    let base_gas_for_use = if let Some(usage) = usage_config {
        match usage.permissions {
            Some(ClaimPermissions::claim) => {
                BASE_GAS_FOR_CLAIM
            }
            _ => BASE_GAS_FOR_CREATE_ACC_AND_CLAIM
        }
    } else {
        BASE_GAS_FOR_CREATE_ACC_AND_CLAIM
    };

    // Check and make sure that the time config is valid
    if let Some(time_config) = use_config.as_ref().and_then(|c| c.get_time_config()).or(drop_config.as_ref().and_then(|c| c.get_time_config())) {
        assert_valid_time_config(&time_config)
    }

    // Keep track of the total gas across all assets in the current use
    near_sdk::log!("{} base gas for use {}", use_number, base_gas_for_use.0);
    let mut total_gas_for_use: Gas = base_gas_for_use;

    // Loop through each asset metadata and tally the costs
    for metadata in assets_metadata {
        // Get the asset object (we only clear the assets by ID when the drop is empty and deleted)
        let internal_asset = asset_by_id
            .get(&metadata.asset_id)
            .expect("Asset not found");

        // Every asset has a gas cost associated. We should add that to the total gas.
        let gas_for_asset = internal_asset.get_required_gas();
        total_gas_for_use += gas_for_asset;

        // Get the refund amount for the asset
        *total_cost_for_use += internal_asset.get_yocto_refund_amount(&metadata.tokens_per_use.map(|x| x.into()));
    }
    require!(total_gas_for_use <= MAX_GAS_ATTACHABLE, format!("Cannot exceed 300 TGas for any given key use. Found {}", total_gas_for_use.0));

    // Get the total allowance for this use
    *total_allowance_for_use += calculate_base_allowance(YOCTO_PER_GAS, total_gas_for_use, false);
}

/// Take a token ID and return the drop ID and key nonce based on the `:` delimiter.
pub(crate) fn parse_token_id(token_id: &TokenId) -> (DropId, u64) {
    let delimiter = ":";
    let split: Vec<&str> = token_id.split(delimiter).collect();
    let drop_id = split[0];
    let key_nonce = split[1].parse::<u64>().expect("Key nonce is not a valid number");
    return (drop_id.to_string(), key_nonce);
}

/// Helper function to convert an external asset to an internal asset
pub(crate) fn ext_asset_to_internal(ext_asset: Option<&ExtAsset>) -> InternalAsset {
    if let Some(asset) = ext_asset {
        return asset.to_internal_asset();
    }

    return InternalAsset::none;
}