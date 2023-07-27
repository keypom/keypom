use crate::*;

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn add_keys(
        &mut self, 
        drop_id: DropId, 
        key_data: Vec<ExtKeyData>, 
    ) -> bool {
        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);

        // get the drop object (remove it and only re-insert at the end if it shouldn't be deleted)
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");
        
        let funder_id = drop.funder_id.clone();
        require!(
            funder_id == env::predecessor_account_id(),
            "Only drop funder can add keys"
        );
        require!(key_data.len() > 0, "Must provide at least one public key");

        // Parse the external assets and store them in the contract
        let max_key_uses = drop.max_key_uses;

        let mut total_cost_per_key = 0;
        let mut total_allowance_per_key = 0;

        // Get the total cost and allowance required for a key that has all its uses remaining
        // We'll then multiply this by the number of keys we want to add and charge the user
        get_total_costs_for_key(
            &mut total_cost_per_key,
            &mut total_allowance_per_key,
            max_key_uses,
            max_key_uses,
            &drop.asset_by_id,
            drop.key_use_behaviors,
            &drop.drop_config
        );

        // Keep track of all the events
        let mut event_logs = Vec::new();
        // Add the keys to the contract
        self.internal_add_keys_to_account(
            &mut drop.next_key_id,
            &mut drop.key_info_by_token_id,
            &mut event_logs,
            &drop_id,
            max_key_uses,
            &key_data,
            ACCESS_KEY_BOTH_METHOD_NAMES, 
            total_allowance_per_key
        );

        // Write the updated drop data to storage
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure final costs
        let net_storage = env::storage_usage() - initial_storage;
        self.determine_costs(
            key_data.len(),
            total_cost_per_key,
            total_allowance_per_key,
            net_storage,
        );

        // Now that everything is done (no more potential for panics), we can log the events
        log_events(event_logs);

        true
    }
}