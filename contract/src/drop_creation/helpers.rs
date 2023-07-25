use crate::*;

#[near_bindgen]
impl Keypom {
    /// Loops through public keys and adds them to the current contract account
    /// Also adds the keys to the drop_id_for_pk map and ensures that no keys are already on the contract
    pub(crate) fn internal_add_keys_to_account(
        &mut self,
        next_key_id: &mut u64,
        key_info_by_token_id: &mut UnorderedMap<TokenId, InternalKeyInfo>,
        event_logs: &mut Vec<EventLog>,
        drop_id: &DropId,
        max_uses_per_key: UseNumber,
        public_keys: &Vec<PublicKey>,
        metadata_for_keys: Option<Vec<Option<String>>>,
        key_owners: Option<Vec<Option<AccountId>>>,
        method_names: &str, 
        allowance: Balance
    ) {
        let current_account_id = &env::current_account_id();

        // Ensure that a key owner is specified for each key
        let num_pks = public_keys.len();
        require!(key_owners.clone().map(|o| o.len()).unwrap_or(num_pks) == num_pks, "Must specify an owner for each key");
        require!(metadata_for_keys.clone().map(|m| m.len()).unwrap_or(num_pks) == num_pks, "Must specify metadata for each key");

        // Logs for add key and NFT mint events
        let mut add_key_logs = Vec::new();
        let mut nft_mint_logs = Vec::new();

        // Create a new promise batch to create all the access keys
        let promise = env::promise_batch_create(current_account_id);

        // Loop through the public keys and add them to the contract.
        // None of these promises will fire if there's a panic so it's
        // Fine to add them in the loop
        for (i, pk) in public_keys.iter().enumerate() {
            let token_id = format!("{}:{}", drop_id, next_key_id);
            let token_owner = key_owners.as_ref().and_then(|o| o[i].clone()).unwrap_or(env::current_account_id());
            let key_metadata: Option<String> = metadata_for_keys.as_ref().and_then(|m| m[i].clone());
            
            require!(
                self.token_id_by_pk.insert(pk, &token_id).is_none(),
                "Key already added to contract"
            );

            key_info_by_token_id.insert(&token_id, &InternalKeyInfo { 
                pub_key: pk.clone(), 
                remaining_uses: max_uses_per_key,
                owner_id: token_owner.clone(), 
                next_approval_id: 0,
                approved_account_ids: Default::default(),
                metadata: key_metadata.clone()
            });
            
            // Add this key to the batch
            env::promise_batch_action_add_key_with_function_call(
                promise,
                pk,
                0, // Nonce
                allowance,
                current_account_id,
                method_names,
            );

            // Construct the nft mint and add key logs to be added as events later
            nft_mint_logs.push(NftMintLog {
                owner_id: token_owner.to_string(),
                token_ids: vec![token_id.to_string()],
                memo: None,
            });
            add_key_logs.push(AddOrDeleteKeyLog {
                owner_id: token_owner.to_string(),
                drop_id: drop_id.to_string(),
                public_key: pk.into(),
                metadata: key_metadata
            });

            *next_key_id += 1;
        }

        // Construct the events themselves
        event_logs.push(EventLog {
            standard: NFT_STANDARD_NAME.to_string(),
            version: NFT_METADATA_SPEC.to_string(),
            event: EventLogVariant::NftMint(nft_mint_logs),
        });
        event_logs.push(EventLog {
            standard: KEYPOM_STANDARD_NAME.to_string(),
            version: KEYPOM_STANDARD_VERSION.to_string(),
            event: EventLogVariant::AddKey(add_key_logs),
        });

        env::promise_return(promise);
    }

     /// Tally up all the costs for adding keys / creating a drop and refund any excess deposit
     pub(crate) fn determine_costs(
        &mut self,
        num_keys: usize, 
        asset_cost_per_key: Balance, 
        allowance_per_key: Balance,
        net_storage: u64
    ) {
        let num_keys = num_keys as u128;
        let storage_cost = net_storage as Balance * env::storage_byte_cost();
        let total_asset_cost = asset_cost_per_key * num_keys;
        let total_allowance_cost = allowance_per_key * num_keys;
        let total_cost = total_asset_cost + storage_cost + total_allowance_cost;
        near_sdk::log!("total {} storage {} asset {} allowance {}", total_cost, storage_cost, total_asset_cost, total_allowance_cost);
        self.charge_with_deposit_or_balance(total_cost);
    }
}

/// Parses the external assets and stores them in the drop's internal maps
pub fn parse_ext_assets_per_use (
    uses_per_key: UseNumber,
    asset_data_per_use: ExtAssetDataPerUse, 
    key_behavior_by_use: &mut LookupMap<UseNumber, InternalKeyBehavior>,
    asset_by_id: &mut UnorderedMap<AssetId, InternalAsset>
) {
    require!(uses_per_key == asset_data_per_use.len() as UseNumber, "Must specify behavior for all uses");

    // Iterate through the external assets, convert them to internal assets and add them to both lookup maps
    for (use_number, ext_asset_data) in asset_data_per_use {
        let AssetDataForGivenUse {assets, config} = ext_asset_data;

        // Quick sanity check to make sure the use number is valid
        require!(use_number <= uses_per_key, "Invalid use number");
        require!(assets.len() > 0, "Must specify at least one asset per use");

        // Keep track of the metadata for all the assets across each use
        let mut assets_metadata: Vec<AssetMetadata> = Vec::new();

        // If there's assets, loop through and get all the asset IDs while also
        // adding them to the asset_by_id lookup map if they weren't already present
        // If there aren't any assets, the vector will be of length 1
        for ext_asset in assets {
            // If the external asset is of type FCData, the asset ID will be the length of the vector
            // Otherwise, it will be the asset ID specified
            let asset_id = if let Some(ExtAsset::FCAsset(_)) = ext_asset {
                asset_by_id.len().to_string()
            } else {
                ext_asset.as_ref().and_then(|a| Some(a.get_asset_id())).unwrap_or(NONE_ASSET_ID.to_string())
            };
            let tokens_per_use = ext_asset.as_ref().and_then(|a| Some(a.get_tokens_per_use())).unwrap_or(U128(0));

            assets_metadata.push(AssetMetadata {
                asset_id: asset_id.clone(),
                tokens_per_use: tokens_per_use.into()
            });

            // Only insert into the asset ID map if it doesn't already exist
            // If we insert, we should also add the cost to the total asset cost
            if asset_by_id.get(&asset_id).is_none() {
                let internal_asset = ext_asset_to_internal(ext_asset.as_ref());

                asset_by_id.insert(&asset_id, &internal_asset);
            }
        }
        key_behavior_by_use.insert(&use_number, &InternalKeyBehavior {
            assets_metadata,
            config
        });
    }
}