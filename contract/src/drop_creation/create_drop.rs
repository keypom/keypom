use crate::*;

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn create_drop_batch(
        &mut self,
        drop_ids: Vec<DropId>,
        asset_datas: Vec<Vec<ExtAssetDataForUses>>,
        drop_configs: Vec<Option<DropConfig>>,
        change_user_metadata: Option<String>,

        on_success: Option<OnSuccessCallData>,
    ) -> bool {
        // Check if all vectors are of the same length
        require!(
            drop_ids.len() == asset_datas.len() && drop_ids.len() == drop_configs.len(),
            "All vectors must be of the same length"
        );

        // Start with the full attached deposit
        let mut deposit_left = env::attached_deposit().as_yoctonear();
        near_sdk::log!("Initial deposit: {}", deposit_left);

        // Iterate over the inputs and call `create_drop` for each set of elements
        for (i, drop_id) in drop_ids.into_iter().enumerate() {
            let current_asset_data = asset_datas[i].clone(); // Assuming `create_drop` can take Vec<ExtAssetDataForUses>
            let current_drop_config = drop_configs[i].clone();

            // Set the new deposit for the next iteration
            deposit_left = self.internal_create_drop(
                drop_id,
                vec![],
                current_asset_data,
                current_drop_config,
                deposit_left,
            );
            near_sdk::log!("Deposit left after drop {}: {}", i, deposit_left);
        }

        // TODO: implement appending metadata to user's metadata. Ensure result is still JSON stringified
        if let Some(metadata) = change_user_metadata {
            deposit_left = self.internal_modify_user_metadata(Some(metadata), deposit_left, None);
            near_sdk::log!(
                "Deposit left after changing user metadata: {}",
                deposit_left
            );
        }

        // Now that all the drops are created, check refund amounts
        if deposit_left > 0 {
            // Only fire cross-contract call if it exists and the attached deposit is sufficient
            if let Some(success_data) = on_success {
                if deposit_left >= success_data.attached_deposit.0 {
                    // Decrement the attached deposit by the amount used in the call
                    deposit_left -= success_data.attached_deposit.0;

                    // Check if the method is prohibited
                    require!(
                        !DEFAULT_PROHIBITED_FC_METHODS.contains(&success_data.method_name.as_str()),
                        format!(
                            "Method {} is prohibited from being called in an FC drop",
                            success_data.method_name
                        )
                    );

                    // Check if the receiver is valid
                    require!(
                        success_data.receiver_id != env::current_account_id(),
                        "Receiver ID cannot be current Keypom contract."
                    );

                    // Fire the cross-contract call
                    Promise::new(success_data.receiver_id).function_call_weight(
                        success_data.method_name.clone(),
                        success_data.args.into(),
                        NearToken::from_yoctonear(success_data.attached_deposit.0),
                        GAS_FOR_CREATE_ACCOUNT,
                        GasWeight(1),
                    );
                }
            }

            // Refund the excess deposit
            let predecessor = env::predecessor_account_id();
            near_sdk::log!("Refunding {} excess deposit", deposit_left);
            Promise::new(predecessor).transfer(NearToken::from_yoctonear(deposit_left));
            return true;
        }

        true
    }
    #[payable]
    pub fn create_drop(
        &mut self,
        drop_id: DropId,
        key_data: Vec<ExtKeyData>,
        asset_data: Vec<ExtAssetDataForUses>,

        drop_config: Option<DropConfig>,
    ) -> bool {
        self.assert_no_global_freeze();
        // Get the amount of $NEAR that should be refunded out of the user's attached deposit
        let refund_amount = self.internal_create_drop(
            drop_id,
            key_data,
            asset_data,
            drop_config,
            env::attached_deposit().as_yoctonear(),
        );

        if refund_amount > 0 {
            let predecessor = env::predecessor_account_id();
            // If the user wants to keep the excess deposit, just modify the user balance

            near_sdk::log!("Refunding {} excess deposit", refund_amount);
            Promise::new(predecessor).transfer(NearToken::from_yoctonear(refund_amount));
            return true;
        }

        true
    }

    pub(crate) fn internal_create_drop(
        &mut self,
        drop_id: DropId,
        key_data: Vec<ExtKeyData>,
        asset_data: Vec<ExtAssetDataForUses>,

        drop_config: Option<DropConfig>,
        attached_deposit: Balance,
    ) -> Balance {
        self.assert_no_global_freeze();
        require!(!drop_id.contains(':'), "Drop ID cannot contain a colon (:)");
        require!(
            self.drop_by_id.get(&drop_id).is_none(),
            format!("Drop with ID {} already exists", drop_id)
        );

        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);

        // Instantiate the drop data structures
        let mut key_info_by_token_id: UnorderedMap<TokenId, InternalKeyInfo> =
            UnorderedMap::new(StorageKeys::KeyInfoByPk {
                drop_id_hash: hash_string(&drop_id.to_string()),
            });
        // Since these won't have a ton of data, using standard data structures is fine
        let mut asset_by_id: UnorderedMap<AssetId, InternalAsset> =
            UnorderedMap::new(StorageKeys::AssetById {
                drop_id_hash: hash_string(&drop_id.to_string()),
            });
        let mut asset_data_for_uses = vec![];

        require!(
            key_data.len() <= 100,
            "Cannot add more than 100 keys at a time"
        );

        let mut max_key_uses = 0;
        // Parse the external asset data and convert it into the internal representation
        for ext_asset_data in asset_data {
            // Convert the external asset data into the internal asset data
            asset_data_for_uses.push(InternalAssetDataForUses::from(&ext_asset_data));

            // Take the assets and populate the asset_by_id mapping
            store_assets_by_id(&ext_asset_data.assets, &mut asset_by_id);

            max_key_uses += ext_asset_data.uses;
        }

        let mut total_cost_per_key = 0;
        let mut total_allowance_per_key = drop_config
            .as_ref()
            .and_then(|config| config.extra_allowance_per_key)
            .unwrap_or(U128(0))
            .0;
        // Get the total cost and allowance required for a key that has all its uses remaining
        // We'll then multiply this by the number of keys we want to add and charge the user
        get_total_costs_for_key(
            &mut total_cost_per_key,
            &mut total_allowance_per_key,
            max_key_uses,
            &asset_by_id,
            &asset_data_for_uses,
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
            total_allowance_per_key,
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
        require!(
            self.drop_by_id.insert(&drop_id, &drop).is_none(),
            format!("Drop with ID {} already exists", drop_id)
        );
        // Add the drop ID to the list of drops owned by the funder
        self.internal_add_drop_to_funder(&funder_id, &drop_id);

        // Measure final costs
        let net_storage = env::storage_usage() - initial_storage;
        let refund_amount = self.determine_costs(
            key_data.len(),
            true, // We did create a drop here
            total_cost_per_key,
            total_allowance_per_key,
            net_storage,
            attached_deposit,
        );

        // Construct the drop creation log and push it to the event logs
        let drop_creation_event: EventLog = EventLog {
            standard: KEYPOM_STANDARD_NAME.to_string(),
            version: KEYPOM_STANDARD_VERSION.to_string(),
            event: EventLogVariant::DropCreation(CreateOrDeleteDropLog {
                funder_id: funder_id.to_string(),
                drop_id,
            }),
        };
        event_logs.push(drop_creation_event);

        // Fire cross-contract call to all the accounts in the drop config's allowlist if it exists
        // (and a message was passed in)

        // Now that everything is done (no more potential for panics), we can log the events
        log_events(event_logs);
        refund_amount
    }
}
