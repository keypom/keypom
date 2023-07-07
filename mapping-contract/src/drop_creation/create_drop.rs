use near_sdk::collections::LazyOption;

use crate::*;

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn create_drop(
        &mut self, 
        drop_id: String, 
        public_keys: Option<Vec<PublicKey>>, 
        assets_per_use: HashMap<UseNumber, Vec<Option<ExtAsset>>>,
        drop_metadata: Option<DropMetadata>
    ) {
        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);
        let public_keys = public_keys.unwrap_or(Vec::new());
        
        // Instantiate the drop data structures
        let mut key_behavior_by_use: LookupMap<UseNumber, KeyBehavior> = LookupMap::new(StorageKeys::AssetIdsByUse {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });
        let mut asset_by_id: UnorderedMap<AssetId, InternalAsset> = UnorderedMap::new(StorageKeys::AssetById {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });
        let mut key_info_by_pk: UnorderedMap<PublicKey, InternalKeyInfo> = UnorderedMap::new(StorageKeys::KeyInfoByPk {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });

        // Keep track of the total cost of all assets for 1 key
        let mut per_key_cost_from_assets = 0;

        // Keep track of how much allowance each key needs
        let mut total_allowance_required_per_key = 0;
        // Parse the external assets and store them in the contract
        let uses_per_key = assets_per_use.len() as UseNumber;
        parse_ext_assets_per_use(
            uses_per_key, 
            assets_per_use,
            &mut key_behavior_by_use, 
            &mut asset_by_id, 
            &mut total_allowance_required_per_key, 
            &mut per_key_cost_from_assets
        );

        // Keep track of all the key IDs 
        let mut next_key_id = 0;

        // Add the keys to the contract
        self.internal_add_keys_to_account(
            &mut next_key_id,
            &mut key_info_by_pk,
            &drop_id,
            uses_per_key,
            &public_keys, 
            ACCESS_KEY_BOTH_METHOD_NAMES, 
            total_allowance_required_per_key
        );

        // Write the drop data to storage
        let drop = InternalDrop {
            uses_per_key,
            key_behavior_by_use,
            asset_by_id,
            key_info_by_pk,
            next_key_id,
            funder_id: env::predecessor_account_id(),
            metadata: LazyOption::new(
                StorageKeys::DropMetadata {
                    // We get a new unique prefix for the collection
                    drop_id_hash: hash_drop_id(&drop_id.to_string()),
                },
                drop_metadata.as_ref(),
            ),
        };
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure final storage costs
        let final_storage = env::storage_usage();
        let storage_cost = (final_storage - initial_storage) as Balance * env::storage_byte_cost();
        let num_keys = public_keys.len() as Balance;
        let total_asset_cost = per_key_cost_from_assets * num_keys;
        let total_allowance_cost = total_allowance_required_per_key * num_keys;
        let total_cost = total_asset_cost + storage_cost + total_allowance_cost;
        near_sdk::log!("total {} storage {} asset {} allowance {}", total_cost, storage_cost, total_asset_cost, total_allowance_cost);
        internal_refund_excess_deposit(total_cost);
    }

    #[payable]
    pub fn add_keys(
        &mut self, 
        drop_id: String, 
        public_keys: Vec<PublicKey>
    ) {
        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);

        // get the drop object (remove it and only re-insert at the end if it shouldn't be deleted)
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");
        
        let funder_id = drop.funder_id.clone();
        require!(
            funder_id == env::predecessor_account_id(),
            "Only drop funder can add keys"
        );

        // Parse the external assets and store them in the contract
        let uses_per_key = drop.uses_per_key;

        let mut total_cost_per_key = 0;
        let mut total_allowance_per_key = 0;

        // Get the total cost and allowance required for a key that has all its uses remaining
        // We'll then multiply this by the number of keys we want to add and charge the user
        get_total_costs_for_key(
            &mut total_cost_per_key,
            &mut total_allowance_per_key,
            uses_per_key,
            uses_per_key,
            &drop,
        );

        // Add the keys to the contract
        self.internal_add_keys_to_account(
            &mut drop.next_key_id,
            &mut drop.key_info_by_pk,
            &drop_id,
            uses_per_key,
            &public_keys, 
            ACCESS_KEY_BOTH_METHOD_NAMES, 
            total_allowance_per_key
        );

        // Write the updated drop data to storage
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure final storage costs
        let final_storage = env::storage_usage();
        let storage_cost = (final_storage - initial_storage) as Balance * env::storage_byte_cost();
        let num_keys = public_keys.len() as Balance;
        let total_asset_cost = total_cost_per_key * num_keys;
        let total_allowance_cost = total_allowance_per_key * num_keys;
        let total_cost = total_asset_cost + storage_cost + total_allowance_cost;
        near_sdk::log!("total {} storage {} asset {} allowance {}", total_cost, storage_cost, total_asset_cost, total_allowance_cost);
        internal_refund_excess_deposit(total_cost);
    }

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
        allowance: Balance
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

/// Parses the external assets and stores them in the drop's internal maps
pub(crate) fn parse_ext_assets_per_use (
    uses_per_key: UseNumber,
    assets_per_use: HashMap<UseNumber, Vec<Option<ExtAsset>>>, 
    key_behavior_by_use: &mut LookupMap<UseNumber, KeyBehavior>,
    asset_by_id: &mut UnorderedMap<AssetId, InternalAsset>, 
    total_allowance_required_per_key: &mut Balance, 
    per_key_cost_from_assets: &mut Balance
) {
    require!(uses_per_key == assets_per_use.len() as UseNumber, "Must specify behavior for all uses");

    // Iterate through the external assets, convert them to internal assets and add them to both lookup maps
    for (use_number, ext_assets) in assets_per_use {
        // Quick sanity check to make sure the use number is valid
        require!(use_number <= uses_per_key, "Invalid use number");

        // Keep track of the metadata for all the assets across each use
        let mut assets_metadata: Vec<AssetMetadata> = Vec::new();

        // Keep track of the total gas across all assets in a given use
        // Default to account creation Gas on top unless otherwise explicitly set
        let mut total_gas_for_use: Gas = BASE_GAS_FOR_CLAIM + GAS_FOR_CREATE_ACCOUNT;
        
        // If there's assets, loop through and get all the asset IDs while also
        // adding them to the asset_by_id lookup map if they weren't already present
        // If there aren't any assets, the vector will be of length 1
        for ext_asset in ext_assets {
            let asset_id = ext_asset.as_ref().and_then(|a| Some(asset_id_from_ext_asset(&a))).unwrap_or(NONE_ASSET_ID.to_string());
            let tokens_per_use = ext_asset.as_ref().and_then(|a| Some(a.get_tokens_per_use())).unwrap_or(U128(0));

            assets_metadata.push(AssetMetadata {
                asset_id: asset_id.clone(),
                tokens_per_use: tokens_per_use.into()
            });

            // Every asset has a cost associated. We should add that to the total cost.
            // This is for 1 key. At the end, we'll multiply by the number of keys
            let cost_for_asset = ext_asset.as_ref().and_then(|a| Some(a.get_cost_per_key())).unwrap_or(0);
            *per_key_cost_from_assets += cost_for_asset;

            // Every asset has a gas cost associated. We should add that to the total gas.
            let gas_for_asset = ext_asset.as_ref().and_then(|a| Some(a.get_required_gas())).unwrap_or(GAS_FOR_NONE_ASSET);
            total_gas_for_use += gas_for_asset;

            // Only insert into the asset ID map if it doesn't already exist
            // If we insert, we should also add the cost to the total asset cost
            if asset_by_id.get(&asset_id).is_none() {
                let internal_asset = ext_asset_to_internal(ext_asset.as_ref());

                asset_by_id.insert(&asset_id, &internal_asset);
            }
        }

        require!(total_gas_for_use <= MAX_GAS_ATTACHABLE, format!("Cannot exceed 300 TGas for any given key use. Found {}", total_gas_for_use.0));
        // Now that all the assets have been looped for the given use, we can get the allowance required
        *total_allowance_required_per_key += calculate_base_allowance(YOCTO_PER_GAS, total_gas_for_use, true);

        key_behavior_by_use.insert(&use_number, &KeyBehavior {
            assets_metadata,
            config: None
        });
    }
}