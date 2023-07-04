use crate::*;

#[near_bindgen]
impl Keypom {
    /*
        User can pass in a vector of public keys or a drop ID.
        If a drop ID is passed in, it will auto delete up to limit.
        If limit is not specified, auto assume 100 keys from the drop.
        All keys must be from the same drop ID.

        All keys must be unregistered (NFTs / FTs refunded) for the drop.
    */
    pub fn delete_keys(
        &mut self,
        drop_id: DropId,
        public_keys: Option<Vec<PublicKey>>,
        limit: Option<u8>,
        delete_on_empty: Option<bool>,
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

        let max_uses_per_key = drop.uses_per_key;

        // Get a list of all the public keys that should be deleted.
        // This is either what was passed in, or the first limit (or 100) keys in the drop
        let public_keys = public_keys.unwrap_or_else(|| drop.key_info_by_pk.keys().take(limit.unwrap_or(100) as usize).collect());

        // Loop through each public key and delete it
        for pk in &public_keys {
            // Get the key info for this public key (by removing - re-entrancy attack prevention)
            let key_info = drop.key_info_by_pk.remove(pk).expect("Key not found");

            // Get the current use number for this key
            let cur_key_use = max_uses_per_key - key_info.remaining_uses + 1;

            // Get the assets metadata for this use number (by removing - re-entrancy attack prevention)
            let assets_metadata = drop
                .assets_metadata_by_use
                .remove(&cur_key_use)
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

                // Put the asset back in storage
                drop.asset_by_id.insert(&metadata.asset_id, &internal_asset);
            }
        }

        // Loop through and delete keys
        for pk in &public_keys {
            // Create the batch promise
            let promise = env::promise_batch_create(&env::current_account_id());

            env::promise_batch_action_delete_key(promise, &pk);

            env::promise_return(promise);
        }

    }
}