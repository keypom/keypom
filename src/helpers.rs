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
        near_sdk::log!("Refunding {} excess deposit", refund_amount);
        Promise::new(env::predecessor_account_id()).transfer(refund_amount);
    }
}

/// Query for the key's current use number given the drop and key info
pub(crate) fn get_key_cur_use(drop: &InternalDrop, key_info: &InternalKeyInfo) -> UseNumber {
    drop.uses_per_key - key_info.remaining_uses + 1
}

/// Used to calculate the base allowance needed given attached GAS
pub(crate) fn calculate_base_allowance(yocto_per_gas: u128, attached_gas: Gas) -> u128 {
    let prepaid: u64 = attached_gas.0 + GAS_PER_CCC.0;

    // Get the number of CCCs you can make with the attached GAS
    // 5 TGas GAS_PER_CCC
    let calls_with_gas = (prepaid / GAS_PER_CCC.0) as f32;
    // Get the constant used to pessimistically calculate the required allowance
    let pow_outcome = 1.032_f32.powf(calls_with_gas);

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

pub(crate) fn parse_ext_assets_per_use (
    uses_per_key: UseNumber,
    assets_per_use: HashMap<UseNumber, Vec<ExtAsset>>, 
    assets_metadata_by_use: &mut LookupMap<UseNumber, Vec<AssetMetadata>>,
    asset_by_id: &mut UnorderedMap<AssetId, InternalAsset>, 
    total_allowance_required_per_key: &mut u128, 
    per_key_cost_from_assets: &mut u128
) {
    // Iterate through the external assets, convert them to internal assets and add them to both lookup maps
    for (use_number, ext_assets) in assets_per_use {
        // Quick sanity check to make sure the use number is valid
        require!(use_number <= uses_per_key && use_number > 0, "Invalid use number");

        // Keep track of the metadata for all the assets across each use
        let mut assets_metadata: Vec<AssetMetadata> = Vec::new();

        // Keep track of the total gas across all assets in a given use
        let mut total_gas_for_use: Gas = BASE_GAS_FOR_CLAIM;
        
        // If there's assets, loop through and get all the asset IDs while also
        // adding them to the asset_by_id lookup map if they weren't already present
        // If there aren't any assets, the vector will be of length 1
        for ext_asset in ext_assets {
            let asset_id = asset_id_from_ext_asset(&ext_asset);
            let tokens_per_use = ext_asset.get_tokens_per_use();

            assets_metadata.push(AssetMetadata {
                asset_id: asset_id.clone(),
                tokens_per_use: tokens_per_use.into()
            });

            // Every asset has a cost associated. We should add that to the total cost.
            // This is for 1 key. At the end, we'll multiply by the number of keys
            let cost_for_asset = ExtAsset::get_cost_per_key(&ext_asset);
            near_sdk::log!("cost for asset {}", cost_for_asset);
            *per_key_cost_from_assets += cost_for_asset;

            // Every asset has a gas cost associated. We should add that to the total gas.
            let gas_for_asset = ExtAsset::get_gas_for_asset(&ext_asset);
            near_sdk::log!("gas for asset {:?}", gas_for_asset);
            total_gas_for_use += gas_for_asset;

            // Only insert into the asset ID map if it doesn't already exist
            // If we insert, we should also add the cost to the total asset cost
            if asset_by_id.get(&asset_id).is_none() {
                let internal_asset = ext_asset.to_internal_asset();

                asset_by_id.insert(&asset_id, &internal_asset);
            }
        }

        require!(total_gas_for_use <= MAX_GAS_ATTACHABLE, "Cannot exceed 300 TGas for any given key use");
        // Now that all the assets have been looped for the given use, we can get the allowance required
        *total_allowance_required_per_key += calculate_base_allowance(YOCTO_PER_GAS, total_gas_for_use);

        assets_metadata_by_use.insert(&use_number, &assets_metadata);
    }
}

impl Keypom {
    /// Loops through public keys and adds them to the current contract account
    /// Also adds the keys to the drop_id_for_pk map and ensures that no keys are already on the contract
    pub(crate) fn internal_add_keys_to_account(
        &mut self,
        next_key_id: &mut u64,
        key_info_by_pk: &mut UnorderedMap<PublicKey, InternalKeyInfo>,
        drop_id: &DropId,
        max_uses_per_key: UseNumber,
        public_keys: &Vec<PublicKey>, 
        method_names: &str, 
        allowance: u128
    ) {
        let current_account_id = &env::current_account_id();

        // First loop through all the keys and add them to the drop_id_for_pk map
        // This will also ensure that no keys are already on the contract
        for pk in public_keys {
            require!(
                self.drop_id_for_pk.insert(pk, drop_id).is_none(),
                "Key already added to contract"
            );

            key_info_by_pk.insert(pk, &InternalKeyInfo { remaining_uses: max_uses_per_key, key_id: *next_key_id });
            *next_key_id += 1;
        }

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
}