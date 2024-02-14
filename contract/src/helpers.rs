use crate::*;

pub(crate) fn vec_to_64_byte_array(vec: Vec<u8>) -> Option<[u8; 64]> {
    // Check if the string is exactly 64 bytes
    if vec.len() != 64 {
        return None;
    }

    // Explicitly import TryInto trait
    use std::convert::TryInto;

    let array: [u8; 64] = vec
        .try_into() // Try to convert the Vec<u8> into a fixed-size array
        .expect("Vec with incorrect length"); // This expect will never panic due to the above length check

    Some(array)
}

pub(crate) fn pk_to_32_byte_array(pk: &PublicKey) -> Option<&[u8; 32]> {
    let len = pk.as_bytes().len();
    // Check if the string is exactly 32 or 33 bytes
    if len != 32 && len != 33 {
        return None;
    }

    // Explicitly import TryInto trait
    use std::convert::TryInto;

    // if the public key has the prefix appended, remove it to ensure it's 32 bytes
    if len == 33 {
        return pk.as_bytes()[1..33].try_into().ok();
    }

    pk.as_bytes()[0..32].try_into().ok()
}

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_string(string: &String) -> CryptoHash {
    env::sha256_array(string.as_bytes())
}

/// Helper function to convert yoctoNEAR to $NEAR with 7 decimals of precision.
pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
    //10^17 yoctoNEAR (1 NEAR would be 10_000_000). This is to give a precision of 7 decimal places.
    let formatted_near = yocto / 100_000_000_000_000_000;
    formatted_near as f64 / 10_000_000f64
}

/// Check whether an asset ID is function call or not
pub(crate) fn is_fc_asset_id(asset_id: &AssetId) -> bool {
    asset_id.contains(FC_ASSET_PREFIX)
}

/// Query for the key's current use number given the drop and key info
pub(crate) fn get_key_cur_use(drop: &InternalDrop, key_info: &InternalKeyInfo) -> UseNumber {
    drop.max_key_uses - key_info.remaining_uses + 1
}

/// Helper function that returns the total cost for a given key as well as its allowance
/// This key can be partially used or not
pub(crate) fn get_total_costs_for_key(
    total_cost_for_keys: &mut Balance,
    remaining_uses: UseNumber,
    asset_by_id: &UnorderedMap<AssetId, InternalAsset>,
    asset_data_for_uses: &[InternalAssetDataForUses],
) {
    // Get the remaining asset data
    let remaining_asset_data = get_remaining_asset_data(asset_data_for_uses, remaining_uses);

    // For every remaining asset data, we should query the costs and multiply it by the number of uses left
    for asset_data in remaining_asset_data {
        let InternalAssetDataForUses {
            uses,
            config: use_config,
            assets_metadata,
            required_asset_gas,
        } = asset_data;

        // If the config's permission field is set to Claim, the base should be set accordingly. In all other cases, it should be the base for CAAC
        let base_gas_for_use =
            if let Some(perms) = use_config.as_ref().and_then(|c| c.permissions.as_ref()) {
                match perms {
                    ClaimPermissions::claim => BASE_GAS_FOR_CLAIM,
                    _ => BASE_GAS_FOR_CREATE_ACC_AND_CLAIM,
                }
            } else {
                BASE_GAS_FOR_CREATE_ACC_AND_CLAIM
            };

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
            let cost_for_use =
                internal_asset.get_yocto_refund_amount(&metadata.tokens_per_use.map(|x| x.into()));
            *total_cost_for_keys += cost_for_use * uses as u128;
        }

        let total_claim_gas = required_asset_gas.as_gas() + base_gas_for_use.as_gas();
        require!(
            total_claim_gas <= MAX_GAS_ATTACHABLE.as_gas(),
            format!(
                "Total gas {} exceeds the maximum 300 TGas limit",
                total_claim_gas
            )
        );
    }
}

/// Returns a vector of remaining asset datas given the remaining uses for a key.
/// Tests: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=f11c6325055ed73fccd6b5c870dbccc2
pub(crate) fn get_remaining_asset_data(
    asset_data: &[InternalAssetDataForUses],
    remaining_uses: UseNumber,
) -> Vec<InternalAssetDataForUses> {
    let mut uses_traversed = 0;
    let mut remaining_data = vec![];

    for asset in asset_data.iter().rev() {
        uses_traversed += asset.uses;

        if uses_traversed >= remaining_uses {
            let asset_to_push = InternalAssetDataForUses {
                uses: asset.uses - (uses_traversed - remaining_uses),
                config: asset.config.clone(),
                assets_metadata: asset.assets_metadata.clone(),
                required_asset_gas: asset.required_asset_gas,
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
pub(crate) fn get_asset_data_for_specific_use(
    asset_data_for_uses: &[InternalAssetDataForUses],
    use_number: &UseNumber,
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
        required_asset_gas: Gas::from_gas(0),
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

    Ok((drop_id.to_string(), key_nonce.unwrap()))
}

/// Helper function to convert an external asset to an internal asset
pub(crate) fn ext_asset_to_internal(ext_asset: Option<&ExtAsset>) -> InternalAsset {
    if let Some(asset) = ext_asset {
        return asset.to_internal_asset();
    }

    InternalAsset::none
}

/// Add keypom args to output args for a function call
pub(crate) fn add_keypom_args(
    output_args: &mut String,
    keypom_args: Option<KeypomInjectedArgs>,
    account_id: &AccountId,
    drop_id: &DropId,
    key_id: &String,
    funder_id: &AccountId,
) -> Result<(), String> {
    // Add keypom args and set any user markers
    let keypom_args = keypom_args.unwrap_or(KeypomInjectedArgs {
        account_id_field: None,
        drop_id_field: None,
        key_id_field: None,
        funder_id_field: None,
    });

    insert_keypom_arg(
        output_args,
        &keypom_args.account_id_field,
        account_id.to_string(),
    )?;
    insert_keypom_arg(output_args, &keypom_args.drop_id_field, drop_id.to_string())?;
    insert_keypom_arg(output_args, &keypom_args.key_id_field, key_id.to_string())?;
    insert_keypom_arg(
        output_args,
        &keypom_args.funder_id_field,
        funder_id.to_string(),
    )?;

    if output_args.contains("\"keypom_args\"") {
        return Err(
            "Keypom Args detected in client args. Returning and decrementing keys".to_string(),
        );
    }

    output_args.insert_str(
        output_args.len() - 1,
        &format!(
            ",\"keypom_args\":{}",
            near_sdk::serde_json::to_string(&keypom_args).unwrap()
        ),
    );

    Ok(())
}
