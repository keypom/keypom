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
        key_data: &Vec<ExtKeyData>,
        allowance: Balance
    ) {
        let current_account_id = &env::current_account_id();

        // Logs for add key and NFT mint events
        let mut add_key_logs = Vec::new();
        let mut nft_mint_logs = Vec::new();

        // Create a new promise batch to create all the access keys
        let promise = env::promise_batch_create(current_account_id);

        // Loop through the public keys and add them to the contract.
        // None of these promises will fire if there's a panic so it's
        // Fine to add them in the loop
        for data in key_data.iter() {
            let ExtKeyData { public_key, password_by_use, metadata, key_owner } = data;

            let token_id = format!("{}:{}", drop_id, next_key_id);
            require!(
                self.token_id_by_pk.insert(public_key, &token_id).is_none(),
                "Key already added to contract"
            );

            // Iterate through the key_data.password_by_use hash map (if there is one) and decode all the strings to hex
            let pw_by_use: Option<HashMap<UseNumber, Vec<u8>>> = password_by_use.as_ref().map(|p| {
                p.into_iter().map(|(k, v)| {
                    let decoded = hex::decode(v).expect("Invalid hex string");
                    (*k, decoded)
                }).collect()
            });

            let key_owner = key_owner.clone().unwrap_or(env::current_account_id());

            key_info_by_token_id.insert(&token_id, &InternalKeyInfo { 
                pub_key: public_key.clone(), 
                remaining_uses: max_uses_per_key,
                owner_id: key_owner.clone(), 
                next_approval_id: 0,
                approved_account_ids: Default::default(),
                metadata: metadata.clone(),
                pw_by_use,
            });

            // TODO: add to tokens_per_owner
            
            // Add this key to the batch
            env::promise_batch_action_add_key_with_function_call(
                promise,
                public_key,
                0, // Nonce
                allowance,
                current_account_id,
                ACCESS_KEY_METHOD_NAMES,
            );

            // Construct the nft mint and add key logs to be added as events later
            add_new_key_logs(
                &mut nft_mint_logs,
                &mut add_key_logs,
                &key_owner,
                &drop_id,
                &public_key,
                &token_id,
                &metadata
            );

            *next_key_id += 1;
        }

        // Construct the events themselves
        if nft_mint_logs.len() > 0 {
            event_logs.push(EventLog {
                standard: NFT_STANDARD_NAME.to_string(),
                version: NFT_METADATA_SPEC.to_string(),
                event: EventLogVariant::NftMint(nft_mint_logs),
            });
        }
        if add_key_logs.len() > 0 {
            event_logs.push(EventLog {
                standard: KEYPOM_STANDARD_NAME.to_string(),
                version: KEYPOM_STANDARD_VERSION.to_string(),
                event: EventLogVariant::AddKey(add_key_logs),
            });
        }

        env::promise_return(promise);
    }

     /// Tally up all the costs for adding keys / creating a drop and refund any excess deposit
     pub(crate) fn determine_costs(
        &mut self,
        num_keys: usize, 
        asset_cost_per_key: Balance, 
        allowance_per_key: Balance,
        pub_sale_costs: Balance,
        net_storage: u64
    ) {
        let num_keys = num_keys as u128;
        let storage_cost = net_storage as Balance * env::storage_byte_cost();
        let total_asset_cost = asset_cost_per_key * num_keys;
        let total_allowance_cost = allowance_per_key * num_keys;
        let total_cost = total_asset_cost + storage_cost + total_allowance_cost + pub_sale_costs;
        near_sdk::log!("total {} storage {} asset {} allowance {} pub sale costs {}", total_cost, storage_cost, total_asset_cost, total_allowance_cost, pub_sale_costs);
        self.charge_with_deposit_or_balance(total_cost);
    }
}

pub fn parse_ext_assets (
    ext_assets: &Vec<Option<ExtAsset>>,
    assets_metadata: &mut Vec<AssetMetadata>,
    asset_by_id: &mut UnorderedMap<AssetId, InternalAsset>
) {
    for ext_asset in ext_assets {
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
}

pub fn parse_ext_all_use_assets (
    ext_data: &ExtAssetDataForAllUses,
    asset_by_id: &mut UnorderedMap<AssetId, InternalAsset>
) -> InternalAllUseBehaviors {
    require!(ext_data.assets.len() > 0, "Must specify at least one asset");
    
    // Keep track of the metadata for all the assets across each use
    let mut assets_metadata: Vec<AssetMetadata> = Vec::new();
    parse_ext_assets(
        &ext_data.assets,
        &mut assets_metadata,
        asset_by_id
    );

    InternalAllUseBehaviors { 
        assets_metadata, 
        num_uses: ext_data.num_uses
    }
}

pub fn parse_ext_per_use_assets (
    ext_datas: &Vec<ExtAssetDataForGivenUse>,
    asset_by_id: &mut UnorderedMap<AssetId, InternalAsset>
) -> Vec<InternalKeyBehaviorForUse> {
    let mut key_behavior = Vec::new();

    // Iterate through the external assets, convert them to internal assets and add them to both lookup maps
    for ext_data in ext_datas {
        let ExtAssetDataForGivenUse {assets, config} = ext_data;

        // Quick sanity check to make sure the use number is valid
        require!(assets.len() > 0, "Must specify at least one asset per use");

        // Keep track of the metadata for all the assets across each use
        let mut assets_metadata: Vec<AssetMetadata> = Vec::new();

        parse_ext_assets(
            &assets,
            &mut assets_metadata,
            asset_by_id
        );

        key_behavior.push(InternalKeyBehaviorForUse {
            assets_metadata,
            config: config.clone()
        });
    }

    key_behavior
}

/// Parses the external assets and stores them in the drop's internal maps
pub fn parse_ext_asset_data (
    asset_data: &ExtAssetData, 
    asset_by_id: &mut UnorderedMap<AssetId, InternalAsset>
) -> InternalKeyUseBehaviors {
    match asset_data {
        ExtAssetData::AssetsForAllUses(data) => {
            InternalKeyUseBehaviors::AllUses(parse_ext_all_use_assets(data, asset_by_id))
        },
        ExtAssetData::AssetsPerUse(data) => {
            InternalKeyUseBehaviors::PerUse( parse_ext_per_use_assets(data, asset_by_id))
        }
    }
}