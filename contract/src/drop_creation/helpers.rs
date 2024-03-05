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
        key_data: &[ExtKeyData],
    ) {
        // Logs for add key and NFT mint events
        let mut add_key_logs = Vec::new();
        let mut nft_mint_logs = Vec::new();

        // Loop through the public keys and add them to the contract.
        // None of these promises will fire if there's a panic so it's
        // Fine to add them in the loop
        for data in key_data.iter() {
            let ExtKeyData {
                public_key,
                password_by_use,
                metadata,
                key_owner,
            } = data;

            let token_id = format!("{}:{}", drop_id, next_key_id);
            require!(
                self.token_id_by_pk.insert(public_key, &token_id).is_none(),
                "Key already added to contract"
            );

            // Iterate through the key_data.password_by_use hash map (if there is one) and decode all the strings to hex
            let pw_by_use: Option<HashMap<UseNumber, Vec<u8>>> =
                password_by_use.as_ref().map(|p| {
                    p.iter()
                        .map(|(k, v)| {
                            let decoded = hex::decode(v).expect("Invalid hex string");
                            (*k, decoded)
                        })
                        .collect()
                });

            if let Some(owner) = key_owner {
                // Add the NFT key to the owner's list of tokens
                self.internal_add_token_to_owner(owner, &token_id);
            }
            key_info_by_token_id.insert(
                &token_id,
                &InternalKeyInfo {
                    message_nonce: 0,
                    pub_key: public_key.clone(),
                    remaining_uses: max_uses_per_key,
                    owner_id: key_owner.clone(),
                    next_approval_id: 0,
                    last_claimed: 0, // Set to 0 since this will make the key always claimable.
                    approved_account_ids: Default::default(),
                    metadata: metadata.clone(),
                    pw_by_use,
                },
            );

            // Construct the nft mint and add key logs to be added as events later
            add_new_key_logs(
                &mut nft_mint_logs,
                &mut add_key_logs,
                key_owner,
                drop_id,
                public_key,
                &token_id,
            );

            *next_key_id += 1;
        }

        // Construct the events themselves
        if !nft_mint_logs.is_empty() {
            event_logs.push(EventLog {
                standard: NFT_STANDARD_NAME.to_string(),
                version: NFT_METADATA_SPEC.to_string(),
                event: EventLogVariant::NftMint(nft_mint_logs),
            });
        }
        if !add_key_logs.is_empty() {
            event_logs.push(EventLog {
                standard: KEYPOM_STANDARD_NAME.to_string(),
                version: KEYPOM_STANDARD_VERSION.to_string(),
                event: EventLogVariant::AddKey(add_key_logs),
            });
        }
    }

    /// Tally up all the costs for adding keys / creating a drop and refund any excess deposit
    pub(crate) fn determine_costs(
        &mut self,
        num_keys: usize,
        did_create_drop: bool,
        asset_cost_per_key: Balance,
        net_storage: u64,
        keep_excess_deposit: Option<bool>,
    ) -> Balance {
        let num_keys = num_keys as u128;

        let storage_cost = net_storage as Balance * env::storage_byte_cost().as_yoctonear();
        let total_asset_cost = asset_cost_per_key * num_keys;
        let fees_for_user = self
            .fees_per_user
            .get(&env::predecessor_account_id())
            .unwrap_or(self.fee_structure.clone());
        let total_fees =
            num_keys * fees_for_user.per_key + did_create_drop as u128 * fees_for_user.per_drop;
        self.fees_collected += total_fees;
        let total_cost = total_asset_cost + storage_cost + total_fees;

        near_sdk::log!(
            "total {} storage {} asset {} keypom fees {}",
            total_cost,
            storage_cost,
            total_asset_cost,
            total_fees
        );
        self.charge_with_deposit_or_balance(total_cost, keep_excess_deposit);
        total_cost
    }

    /// Internal method to add a drop ID the list of drops a funder has. If they don't have any, instantiate
    /// A new unordered set and add the drop ID to it. Otherwise, just add the drop ID to the existing set
    pub(crate) fn internal_add_drop_to_funder(&mut self, funder_id: &AccountId, drop_id: &DropId) {
        let mut drop_set = self.drop_ids_by_funder.get(funder_id).unwrap_or_else(|| {
            // If the account doesn't have any drops, we create a new unordered set
            UnorderedSet::new(StorageKeys::DropIdsByFunderInner {
                // We get a new unique prefix for the collection equal to the funder (since it's unique)
                account_id_hash: hash_string(&funder_id.to_string()),
            })
        });

        // We insert the drop ID into the set
        drop_set.insert(drop_id);

        // We insert that set for the given account ID.
        self.drop_ids_by_funder.insert(funder_id, &drop_set);
    }

    /// Internal method to remove a drop ID from the list of drops a funder has.
    /// If the funder has no more drops, we remove the funder from the drop_ids_by_funder collection
    pub(crate) fn internal_remove_drop_for_funder(
        &mut self,
        funder_id: &AccountId,
        drop_id: &DropId,
    ) {
        // Get the set of drop IDs that the funder currently has
        let mut drop_set = self
            .drop_ids_by_funder
            .get(funder_id)
            .expect("No Drops found for the funder");

        // Remove the drop ID from the set
        drop_set.remove(drop_id);

        // If the set is now empty, we remove the funder from the collection
        if drop_set.is_empty() {
            self.drop_ids_by_funder.remove(funder_id);
        } else {
            self.drop_ids_by_funder.insert(funder_id, &drop_set);
        }
    }
}

/// Helper function to ingest external assets and store them in the internal asset by ID map
pub fn store_assets_by_id(
    ext_assets: &Vec<Option<ExtAsset>>,
    asset_by_id: &mut UnorderedMap<AssetId, InternalAsset>,
) {
    let mut fc_idx = 0;
    for ext_asset in ext_assets {
        // If the external asset is of type FCData, the asset ID will be the incrementing number
        // Otherwise, it will be the asset ID specified
        let asset_id = if let Some(ExtAsset::FCAsset(_)) = ext_asset {
            fc_idx += 1;
            format!("{}{}", FC_ASSET_PREFIX, fc_idx)
        } else {
            ext_asset
                .as_ref()
                .map(|a| a.get_asset_id())
                .unwrap_or(NONE_ASSET_ID.to_string())
        };

        // Only insert into the asset ID map if it doesn't already exist
        // If we insert, we should also add the cost to the total asset cost
        if asset_by_id.get(&asset_id).is_none() {
            let internal_asset = ext_asset_to_internal(ext_asset.as_ref());

            asset_by_id.insert(&asset_id, &internal_asset);
        }
    }
}

/// Ensure that the time configurations passed in is valid
pub(crate) fn assert_valid_time_config(config: &TimeConfig) {
    // Assert that if the claim_interval is some, the start_timestamp is also some
    assert!(
        !(config.interval.is_some() && config.start.is_none()),
        "If you want to set a claim interval, you must also set a start timestamp"
    );

    // Assert that both the start_timestamp and end timestamps are greater than the current block
    assert!(
        config.start.unwrap_or(env::block_timestamp()) >= env::block_timestamp(),
        "The start timestamp must be greater than the current block timestamp"
    );
    assert!(
        config.end.unwrap_or(env::block_timestamp()) >= env::block_timestamp(),
        "The end timestamp must be greater than the current block timestamp"
    );

    // If both the start timestamp and end timestamp are set, ensure that the start timestamp is less than the end timestamp
    if config.start.is_some() && config.end.is_some() {
        assert!(
            config.start.unwrap() < config.end.unwrap(),
            "The start timestamp must be less than the end timestamp"
        );
    }
}
