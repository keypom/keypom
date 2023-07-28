use crate::*;

#[near_bindgen]
impl Keypom {
    /*
        User can pass in a vector of public keys or a drop ID.
        If a drop ID is passed in, it will auto delete up to limit.
        If limit is not specified, auto assume 100 keys from the drop.
        All keys must be from the same drop ID.
    */
    pub fn delete_keys(
        &mut self,
        drop_id: DropId,
        public_keys: Option<Vec<PublicKey>>,
        limit: Option<u8>,
        keep_empty_drop: Option<bool>
    ) {
        self.asset_no_global_freeze();

        // Measure initial storage before doing any operations
        let initial_storage = env::storage_usage();

        // get the drop object (remove it and only re-insert at the end if it shouldn't be deleted)
        let mut drop = self.drop_by_id.remove(&drop_id).expect("No drop found");
        
        let funder_id = drop.funder_id.clone();
        require!(
            funder_id == env::predecessor_account_id(),
            "Only drop funder can delete keys"
        );

        // Loop through all the assets in the drop and ensure they're empty
        for asset in drop.asset_by_id.values() {
            require!(
                asset.is_empty(),
                "Withdraw all assets in the drop before deleting keys"
            );
        }

        // Get a list of all the public keys that should be deleted.
        // This is either what was passed in, or the first limit (or 100) keys in the drop
        let public_keys = public_keys.unwrap_or_else(|| drop.key_info_by_token_id.iter().take(limit.unwrap_or(100).into()).map(|k| k.1.pub_key).collect());
        
        
        // Create the batch promise for deleting the keys
        let key_deletion_promise = env::promise_batch_create(&env::current_account_id());
        
        // Keep track of the total cost for the key & the required allowance to be refunded
        let mut total_cost_for_keys: Balance = 0;
        let mut total_allowance_for_keys: Balance = 0;

        let mut delete_key_logs = Vec::new();
        let mut nft_burn_logs = Vec::new();

        // Loop through each public key and delete it
        for pk in &public_keys {
            // Get the key info for this public key (by removing - re-entrancy attack prevention)
            let token_id = self.token_id_by_pk.remove(pk).expect("Token ID not found for Public Key");
            let key_info = drop.key_info_by_token_id.remove(&token_id).expect("Key Info not found for Token ID");
            self.internal_remove_token_from_owner(&key_info.owner_id, &token_id);

            // For every remaining use, we need to loop through all assets and refund
            get_total_costs_for_key(
                &mut total_cost_for_keys,
                &mut total_allowance_for_keys,
                key_info.remaining_uses,
                drop.max_key_uses,
                &drop.asset_by_id,
                &drop.key_use_behaviors,
                &drop.drop_config
            );

            add_delete_key_logs(
                &mut nft_burn_logs,
                &mut delete_key_logs,
                &key_info.owner_id,
                &drop_id,
                &pk,
                &token_id,
                &key_info.metadata
            );

            // Add the delete key action to the batch promise
            env::promise_batch_action_delete_key(key_deletion_promise, &pk);
        }

        // Keep track of all the events starting with the NFT burn and key deletion logs
        let mut event_logs = vec![
            EventLog {
                standard: NFT_STANDARD_NAME.to_string(),
                version: NFT_METADATA_SPEC.to_string(),
                event: EventLogVariant::NftBurn(nft_burn_logs),
            },
            EventLog {
                standard: KEYPOM_STANDARD_NAME.to_string(),
                version: KEYPOM_STANDARD_VERSION.to_string(),
                event: EventLogVariant::DeleteKey(delete_key_logs),
            }
        ];
        
        if drop.key_info_by_token_id.is_empty() && !keep_empty_drop.unwrap_or(false) {
            // Now that the drop is empty, we can delete the assets by use and asset by ID
            // The drop has already been removed from storage, so we can just clear the maps
            internal_clear_drop_storage(&mut drop, &mut event_logs, &drop_id);
            self.internal_remove_drop_for_funder(&funder_id, &drop_id);
        } else {
            // Put the modified drop back in storage
            self.drop_by_id.insert(&drop_id, &drop);
        }

        // Measure the final storage released after all operations
        let storage_released = initial_storage - env::storage_usage();
        let storage_refund = storage_released as u128 * env::storage_byte_cost();
        
        let total_refund_for_use = total_cost_for_keys + total_allowance_for_keys + storage_refund;
        near_sdk::log!("Allowance Refund: {} Cost Refund: {} Storage Refund: {}", total_allowance_for_keys, total_cost_for_keys, storage_refund);
        self.internal_modify_user_balance(&funder_id, total_refund_for_use, false);

        // Now that everything is done (no more potential for panics), we can log the events
        log_events(event_logs);

        env::promise_return(key_deletion_promise);
    }
}

/// Internal helper function to clear the storage in the drop
pub(crate) fn internal_clear_drop_storage(
    drop: &mut InternalDrop,
    event_logs: &mut Vec<EventLog>,
    drop_id: &DropId
) {
    drop.asset_by_id.clear();

    // Add the drop deletion log to the event logs
    event_logs.push(EventLog {
        standard: KEYPOM_STANDARD_NAME.to_string(),
        version: KEYPOM_STANDARD_VERSION.to_string(),
        event: EventLogVariant::DropDeletion(DropDeletionLog {
            funder_id: drop.funder_id.to_string(),
            drop_id: drop_id.to_string(),
            metadata: drop.metadata.get()
        })
    });
}