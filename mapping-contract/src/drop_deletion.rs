use crate::*;

#[near_bindgen]
impl Keypom {
    /// Delete an empty drop
    pub fn delete_drop(
        &mut self,
        drop_id: DropId
    ) {
        // Measure initial storage before doing any operations
        let initial_storage = env::storage_usage();

        // get the drop object
        let mut drop = self.drop_by_id.remove(&drop_id).expect("No drop found");
        
        let funder_id = &drop.funder_id;
        require!(
            funder_id == &env::predecessor_account_id(),
            "Only drop funder can delete keys"
        );

        require!(drop.key_info_by_pk.is_empty(), "Drop must be empty to delete");
        
        // Now that the drop is empty, we can delete the assets by use and asset by ID
        // The drop has already been removed from storage, so we can just clear the maps
        drop.asset_by_id.clear();
        clear_asset_metadata_map(drop.uses_per_key, &mut drop.assets_metadata_by_use);

        // Measure the final storage released after all operations
        let storage_released = initial_storage - env::storage_usage();
        self.internal_modify_user_balance(&funder_id, storage_released as u128 * env::storage_byte_cost(), false);
    }

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
        limit: Option<u8>
    ) {
        // Measure initial storage before doing any operations
        let initial_storage = env::storage_usage();

        // get the drop object
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");
        let funder_id = &drop.funder_id;

        require!(
            funder_id == &env::predecessor_account_id(),
            "Only drop funder can delete keys"
        );

        // Get a list of all the public keys that should be deleted.
        // This is either what was passed in, or the first limit (or 100) keys in the drop
        let public_keys = public_keys.unwrap_or_else(|| drop.key_info_by_pk.keys().take(limit.unwrap_or(100) as usize).collect());

        // Create the batch promise for deleting the keys
        let key_deletion_promise = env::promise_batch_create(&env::current_account_id());
        // Loop through each public key and delete it
        for pk in &public_keys {
            // Get the key info for this public key (by removing - re-entrancy attack prevention)
            let key_info = drop.key_info_by_pk.remove(pk).expect("Key not found");
            
            // For every remaining use, we need to loop through all assets and refund
            for cur_use in 1..=key_info.remaining_uses {                
                // Get the assets metadata for this use number (we only clear the map when the drop is empty and deleted)
                let assets_metadata = drop
                    .assets_metadata_by_use
                    .get(&cur_use)
                    .expect("Use number not found");
    
                // Loop through each asset metadata and delete it
                for metadata in assets_metadata {
                    // Get the asset object (we only clear the assets by ID when the drop is empty and deleted)
                    let mut internal_asset = drop
                        .asset_by_id
                        .get(&metadata.asset_id)
                        .expect("Asset not found");
    
                    // Delete the asset
                    internal_asset.refund_funder(&drop_id, funder_id, metadata.tokens_per_use.map(|x| x.into()));
    
                    // Put the asset back in storage. The internal balances for the asset structs
                    // Will have decremented by the time the promise is fired so there is no re-entrancy attack
                    drop.asset_by_id.insert(&metadata.asset_id, &internal_asset);
                }
            }

            // Add the delete key action to the batch promise
            env::promise_batch_action_delete_key(key_deletion_promise, &pk);
        }

        // Put the modified drop back in storage
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure the final storage released after all operations
        let storage_released = initial_storage - env::storage_usage();
        self.internal_modify_user_balance(&funder_id, storage_released as u128 * env::storage_byte_cost(), false);

        env::promise_return(key_deletion_promise);
    }
}

/// Loop through each use number and remove the assets metadata for that use number
pub(crate) fn clear_asset_metadata_map (
    uses_per_key: UseNumber,
    assets_metadata_by_use: &mut LookupMap<UseNumber, Vec<AssetMetadata>>,
) {
    for use_number in 1..=uses_per_key {
        assets_metadata_by_use.remove(&use_number);
    }
}