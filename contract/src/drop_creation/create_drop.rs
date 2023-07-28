use crate::*;
use near_sdk::collections::LazyOption;

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn create_drop(
        &mut self, 
        drop_id: DropId,
        key_data: Option<Vec<ExtKeyData>>, 
        asset_data: ExtAssetData,

        drop_data: Option<ExtDropData>,
    ) -> bool {
        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);

        // Instantiate the drop data structures
        let mut asset_by_id: UnorderedMap<AssetId, InternalAsset> = UnorderedMap::new(StorageKeys::AssetById {
            drop_id_hash: hash_string(&drop_id.to_string()),
        });
        let mut key_info_by_token_id: UnorderedMap<TokenId, InternalKeyInfo> = UnorderedMap::new(StorageKeys::KeyInfoByPk {
            drop_id_hash: hash_string(&drop_id.to_string()),
        });

        let ExtDropData { config: drop_config, metadata: drop_metadata, nft_keys_config } = drop_data.unwrap_or(ExtDropData {
            config: None,
            metadata: None,
            nft_keys_config: None
        });
        let key_data = key_data.unwrap_or(vec![]);

        // Parse the external asset data and convert it into the internal representation
        let key_use_behaviors = parse_ext_asset_data(
            &asset_data,
            &mut asset_by_id 
        );

        let max_key_uses = match &key_use_behaviors {
            InternalKeyUseBehaviors::AllUses(data) => data.num_uses,
            InternalKeyUseBehaviors::PerUse(data) => data.len() as UseNumber,
        };

        let mut total_cost_per_key = 0;
        let mut total_allowance_per_key = 0;
        // Get the total cost and allowance required for a key that has all its uses remaining
        // We'll then multiply this by the number of keys we want to add and charge the user
        get_total_costs_for_key(
            &mut total_cost_per_key,
            &mut total_allowance_per_key,
            max_key_uses,
            max_key_uses,
            &asset_by_id,
            &key_use_behaviors,
            &drop_config
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
            max_key_uses,
            &key_data, 
            total_allowance_per_key
        );

        // Write the drop data to storage
        let funder_id = env::predecessor_account_id();
        let drop = InternalDrop {
            max_key_uses,
            key_use_behaviors,
            asset_by_id,
            key_info_by_token_id,
            next_key_id,
            nft_keys_config,
            drop_config,
            funder_id: funder_id.clone(),
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
            key_data.len(),
            total_cost_per_key,
            total_allowance_per_key,
            net_storage,
        );

        // Construct the drop creation log and push it to the event logs
        let drop_creation_event: EventLog = EventLog {
            standard: KEYPOM_STANDARD_NAME.to_string(),
            version: KEYPOM_STANDARD_VERSION.to_string(),
            event: EventLogVariant::DropCreation(DropCreationLog {
                funder_id: funder_id.to_string(),
                drop_id,
                max_key_uses,
                metadata: drop_metadata
            }),
        };
        event_logs.push(drop_creation_event);

        // Now that everything is done (no more potential for panics), we can log the events
        log_events(event_logs);

        true
    }
}