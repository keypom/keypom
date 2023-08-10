use crate::*;

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn create_drop(
        &mut self, 
        drop_id: DropId,
        key_data: Vec<ExtKeyData>, 
        asset_data: Vec<ExtAssetDataForUses>,

        drop_config: Option<DropConfig>,
        // Should any excess attached deposit be deposited to the user's balance?
        keep_excess_deposit: Option<bool>
    ) -> bool {
        self.assert_no_global_freeze();

        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);

        // Instantiate the drop data structures
        let mut key_info_by_token_id: UnorderedMap<TokenId, InternalKeyInfo> = UnorderedMap::new(StorageKeys::KeyInfoByPk {
            drop_id_hash: hash_string(&drop_id.to_string()),
        });
        // Since these won't have a ton of data, using standard data structures is fine
        let mut asset_by_id: UnorderedMap<AssetId, InternalAsset> = UnorderedMap::new(StorageKeys::AssetById {
            drop_id_hash: hash_string(&drop_id.to_string()),
        });
        let mut asset_data_for_uses = vec![];

        require!(key_data.len() <= 100, "Cannot add more than 100 keys at a time");

        let mut max_key_uses = 0;
        // Parse the external asset data and convert it into the internal representation
        for ext_asset_data in asset_data {
            // Convert the external asset data into the internal asset data
            asset_data_for_uses.push(InternalAssetDataForUses::from(&ext_asset_data));

            // Take the assets and populate the asset_by_id mapping
            store_assets_by_id(
                &ext_asset_data.assets,
                &mut asset_by_id,
            );

            max_key_uses += ext_asset_data.uses;
        }

        let mut total_cost_per_key = 0;
        let mut total_allowance_per_key = 0;
        // Get the total cost and allowance required for a key that has all its uses remaining
        // We'll then multiply this by the number of keys we want to add and charge the user
        get_total_costs_for_key(
            &mut total_cost_per_key,
            &mut total_allowance_per_key,
            max_key_uses,
            &asset_by_id,
            &asset_data_for_uses
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
            asset_data_for_uses,
            asset_by_id,
            key_info_by_token_id,
            next_key_id,
            config: drop_config,
            funder_id: funder_id.clone(),
        };
        require!(self.drop_by_id.insert(&drop_id, &drop).is_none(), format!("Drop with ID {} already exists", drop_id));
        // Add the drop ID to the list of drops owned by the funder
        self.internal_add_drop_to_funder(&funder_id, &drop_id);

        // Measure final costs
        let net_storage = env::storage_usage() - initial_storage;
        self.determine_costs(
            key_data.len(),
            true, // We did create a drop here
            total_cost_per_key,
            total_allowance_per_key,
            net_storage,
            keep_excess_deposit
        );

        // Construct the drop creation log and push it to the event logs
        let drop_creation_event: EventLog = EventLog {
            standard: KEYPOM_STANDARD_NAME.to_string(),
            version: KEYPOM_STANDARD_VERSION.to_string(),
            event: EventLogVariant::DropCreation(CreateOrDeleteDropLog {
                funder_id: funder_id.to_string(),
                drop_id
            }),
        };
        event_logs.push(drop_creation_event);

        // Now that everything is done (no more potential for panics), we can log the events
        log_events(event_logs);

        true
    }
}