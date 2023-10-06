use serde_json::{Value, to_string, from_str};

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

/// Check whether an asset ID is function call or not
pub(crate) fn is_fc_asset_id(asset_id: &AssetId) -> bool {
    asset_id.contains(FC_ASSET_PREFIX)
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
    asset_by_id: &UnorderedMap<AssetId, InternalAsset>,
    asset_data_for_uses: &Vec<InternalAssetDataForUses>,
) {
    // Get the remaining asset data
    let remaining_asset_data = get_remaining_asset_data(asset_data_for_uses, remaining_uses);

    // For every remaining asset data, we should query the costs and multiply it by the number of uses left
    for asset_data in remaining_asset_data {
        let InternalAssetDataForUses { uses, config: use_config, assets_metadata, required_asset_gas } = asset_data;

        // If the config's permission field is set to Claim, the base should be set accordingly. In all other cases, it should be the base for CAAC
        let base_gas_for_use = if let Some(perms) = use_config.as_ref().and_then(|c| c.permissions.as_ref()) {
            match perms {
                ClaimPermissions::claim => {
                    BASE_GAS_FOR_CLAIM
                }
                _ => BASE_GAS_FOR_CREATE_ACC_AND_CLAIM
            }
        } else {
            BASE_GAS_FOR_CREATE_ACC_AND_CLAIM
        };

        // near_sdk::log!("Base gas for use: {}", base_gas_for_use.0);

        // Check and make sure that the time config is valid
        if let Some(time_config) = use_config.as_ref().and_then(|c| c.time.as_ref()) {
            assert_valid_time_config(time_config)
        }

        // Loop through each asset metadata and tally the costs
        for metadata in assets_metadata {
            // Get the asset object (we only clear the assets by ID when the drop is empty and deleted)
            let internal_asset = asset_by_id
                .get(&metadata.asset_id)
                .expect("Asset not found");

            // Get the refund amount for the asset
            let cost_for_use = internal_asset.get_yocto_refund_amount(&metadata.tokens_per_use.map(|x| x.into()));
            *total_cost_for_keys += cost_for_use * uses as u128;
        }

        let total_claim_gas = required_asset_gas + base_gas_for_use;
        near_sdk::log!("TOTAL GAS CALCULATED IN CREATION: {:?}", total_claim_gas);
        require!(total_claim_gas <= MAX_GAS_ATTACHABLE, format!("Total gas {} exceeds the maximum 300 TGas limit", total_claim_gas.0));
        
        // Get the total allowance for this use
        let allowance_for_use = calculate_base_allowance(YOCTO_PER_GAS, total_claim_gas, false);
        *total_allowance_for_keys += allowance_for_use * uses as u128;
    }
}

/// Returns a vector of remaining asset datas given the remaining uses for a key.
/// Tests: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=f11c6325055ed73fccd6b5c870dbccc2
pub(crate) fn get_remaining_asset_data(asset_data: &Vec<InternalAssetDataForUses>, remaining_uses: UseNumber) -> Vec<InternalAssetDataForUses> {
    let mut uses_traversed = 0;
    let mut remaining_data = vec![];
    
    for asset in asset_data.iter().rev() {
        uses_traversed += asset.uses;
        
        if uses_traversed >= remaining_uses {
            let asset_to_push = InternalAssetDataForUses { 
                uses: asset.uses - (uses_traversed - remaining_uses),
                config: asset.config.clone(), 
                assets_metadata: asset.assets_metadata.clone(),
                required_asset_gas: asset.required_asset_gas
            };
            
            remaining_data.push(asset_to_push);
            break;
        } else {
            remaining_data.push(asset.clone());
        }
    }
    
    remaining_data
}

/// Helper function to get the internal key behavior for a given use number
/// Tests: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=e60e0bd12e87b90d375040d3c2fad715
pub(crate) fn get_asset_data_for_specific_use (
    asset_data_for_uses: &Vec<InternalAssetDataForUses>,
    use_number: &UseNumber
) -> InternalAssetDataForUses {
    let mut cur_use = 0;

    for asset_data in asset_data_for_uses.iter() {
        cur_use += asset_data.uses;

        if cur_use >= *use_number {
            return asset_data.clone();
        }
    }

    InternalAssetDataForUses {
        uses: 0,
        assets_metadata: vec![],
        config: None,
        required_asset_gas: Gas(0)
    }
}

/// Take a token ID and return the drop ID and key nonce based on the `:` delimiter.
pub(crate) fn parse_token_id(token_id: &TokenId) -> Result<(DropId, u64), String> {
    let delimiter = ":";
    let split: Vec<&str> = token_id.split(delimiter).collect();
    let drop_id = split[0];
    let key_nonce = split[1].parse::<u64>();
    if key_nonce.is_err() {
        return Err("Invalid key nonce".to_string());
    }

    return Ok((drop_id.to_string(), key_nonce.unwrap()));
}

/// Helper function to convert an external asset to an internal asset
pub(crate) fn ext_asset_to_internal(ext_asset: Option<&ExtAsset>) -> InternalAsset {
    if let Some(asset) = ext_asset {
        return asset.to_internal_asset();
    }

    return InternalAsset::none;
}

/// Add keypom args to output args for a function call
pub(crate) fn add_keypom_args(
    output_args: &mut String, 
    keypom_args: Option<KeypomInjectedArgs>, 
    account_id: &AccountId,
    drop_id: &DropId,
    key_id: &String,
    funder_id: &AccountId,
    can_nest: bool
) -> Result<(), String> {
    // Add keypom args and set any user markers
    let keypom_args = keypom_args.unwrap_or(KeypomInjectedArgs { 
        account_id_field: None,
        drop_id_field: None,
        key_id_field: None,
        funder_id_field: None
    });

    if can_nest {
        let try_json: Result<Value, _> = from_str(&output_args);
        if try_json.is_err() {
            return Err("Cannot cast args to JSON. Returning and decrementing keys".to_string());
        }
        let mut output_args_json = try_json.unwrap();

        insert_nested_keypom_arg(
            &mut output_args_json,
            &keypom_args.account_id_field,
            account_id.to_string()
        )?;
        insert_nested_keypom_arg(
            &mut output_args_json,
            &keypom_args.drop_id_field,
            drop_id.to_string()
        )?;
        insert_nested_keypom_arg(
            &mut output_args_json,
            &keypom_args.key_id_field,
            key_id.to_string()
        )?;
        insert_nested_keypom_arg(
            &mut output_args_json,
            &keypom_args.funder_id_field,
            funder_id.to_string()
        )?;

        *output_args = to_string(&output_args_json).unwrap();
    } else {
        insert_top_level_keypom_arg(
            output_args,
            &keypom_args.account_id_field,
            account_id.to_string()
        )?;
        insert_top_level_keypom_arg(
            output_args,
            &keypom_args.drop_id_field,
            drop_id.to_string()
        )?;
        insert_top_level_keypom_arg(
            output_args,
            &keypom_args.key_id_field,
            key_id.to_string()
        )?;
        insert_top_level_keypom_arg(
            output_args,
            &keypom_args.funder_id_field,
            funder_id.to_string()
        )?;
    }

    if output_args.contains("\"keypom_args\"") {
        return Err("Keypom Args detected in client args. Returning and decrementing keys".to_string());
    }

    output_args.insert_str(
        output_args.len() - 1,
        &format!(
            ",\"keypom_args\":{}",
            near_sdk::serde_json::to_string(&keypom_args).unwrap()
        ),
    );

    return Ok(());
}