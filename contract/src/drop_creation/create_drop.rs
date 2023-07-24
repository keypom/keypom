use crate::*;
use near_sdk::collections::LazyOption;

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn create_drop(
        &mut self, 
        drop_id: String, 
        public_keys: Option<Vec<PublicKey>>, 
        
        asset_data_per_use: Option<ExtAssetDataPerUse>,
        asset_data_for_all_uses: Option<ExtAssetDataForAllUses>,

        config_for_all_uses: Option<ConfigForAllUses>,

        drop_metadata: Option<DropMetadata>,
        nft_config: Option<NFTKeyBehaviour>,
        
        // What will the owners of the keys be? Must match length of public keys
        key_owners: Option<Vec<Option<AccountId>>>
    ) -> bool {
        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);
        let public_keys = public_keys.unwrap_or(Vec::new());

        // Instantiate the drop data structures
        let mut key_behavior_by_use: LookupMap<UseNumber, InternalKeyBehavior> = LookupMap::new(StorageKeys::AssetIdsByUse {
            drop_id_hash: hash_string(&drop_id.to_string()),
        });
        let mut asset_by_id: UnorderedMap<AssetId, InternalAsset> = UnorderedMap::new(StorageKeys::AssetById {
            drop_id_hash: hash_string(&drop_id.to_string()),
        });
        let mut key_info_by_token_id: UnorderedMap<TokenId, InternalKeyInfo> = UnorderedMap::new(StorageKeys::KeyInfoByPk {
            drop_id_hash: hash_string(&drop_id.to_string()),
        });

        // If there were assets for each use, convert them to assets per use hash map
        let actual_asset_data_per_use = asset_data_per_use.unwrap_or_else(|| {
            let ExtAssetDataForAllUses {num_uses, assets } = asset_data_for_all_uses.expect("Must provide asset_data_for_all_uses if asset_data_per_use is not provided");
            
            // If the funder specified assets for all the users
            let asset_data_for_use = AssetDataForGivenUse {
                assets,
                config: None
            };

            // Loop from 1 -> num_uses and add the assets to the hashmap
            let mut tmp_assets_per_use = HashMap::new();
            for use_number in 1..=num_uses {
                tmp_assets_per_use.insert(use_number, asset_data_for_use.clone());
            }

            tmp_assets_per_use
        });

        // Parse the external assets and store them in the contract
        let uses_per_key = actual_asset_data_per_use.len() as UseNumber;
        parse_ext_assets_per_use(
            uses_per_key, 
            actual_asset_data_per_use,
            &mut key_behavior_by_use, 
            &mut asset_by_id 
        );


        let mut total_cost_per_key = 0;
        let mut total_allowance_per_key = 0;

        // Get the total cost and allowance required for a key that has all its uses remaining
        // We'll then multiply this by the number of keys we want to add and charge the user
        get_total_costs_for_key(
            &mut total_cost_per_key,
            &mut total_allowance_per_key,
            uses_per_key,
            uses_per_key,
            &asset_by_id,
            &key_behavior_by_use
        );

        // Keep track of all the key IDs 
        let mut next_key_id = 0;
        // Keep track of all the events
        let mut event_logs = Vec::new();
        // Add the keys to the contract
        self.internal_add_keys_to_account(
            &mut next_key_id,
            &mut key_info_by_token_id,
            &mut event_logs,
            &drop_id,
            uses_per_key,
            &public_keys, 
            key_owners,
            ACCESS_KEY_BOTH_METHOD_NAMES, 
            total_allowance_per_key
        );

        // Write the drop data to storage
        let drop = InternalDrop {
            uses_per_key,
            key_behavior_by_use,
            asset_by_id,
            key_info_by_token_id,
            next_key_id,
            nft_config,
            drop_config: config_for_all_uses,
            funder_id: env::predecessor_account_id(),
            metadata: LazyOption::new(
                StorageKeys::DropMetadata {
                    // We get a new unique prefix for the collection
                    drop_id_hash: hash_string(&drop_id.to_string()),
                },
                drop_metadata.as_ref(),
            ),
        };
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure final costs
        let net_storage = env::storage_usage() - initial_storage;
        self.determine_costs(
            public_keys.len(),
            total_cost_per_key,
            total_allowance_per_key,
            net_storage,
        );

        // Now that everything is done (no more potential for panics), we can log the events
        log_events(event_logs);

        true
    }
}