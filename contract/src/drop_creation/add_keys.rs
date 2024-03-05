use crate::*;

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn add_keys(
        &mut self,
        drop_id: DropId,
        key_data: Vec<ExtKeyData>,
        // Should any excess attached deposit be deposited to the user's balance?
        keep_excess_deposit: Option<bool>,
    ) -> bool {
        self.assert_no_global_freeze();

        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);

        // get the drop object (remove it and only re-insert at the end if it shouldn't be deleted)
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");

        let funder_id = drop.funder_id.clone();
        let caller_id = env::predecessor_account_id();

        let num_keys_to_add = key_data.len();
        require!(
            num_keys_to_add > 0 && num_keys_to_add <= 100,
            "Must provide between 1 and 100 keys at a time"
        );

        // If there is a public sale and the predecessor isn't the funder, perform checks and return revenue
        if let Some(allowlist) = drop
            .config
            .as_ref()
            .and_then(|c| c.add_key_allowlist.as_ref())
        {
            if funder_id != caller_id {
                require!(allowlist.contains(&caller_id), "caller not on allowlist");
            }
        } else {
            require!(
                funder_id == caller_id,
                "Only funder can add keys to the drop"
            );
        }

        // Parse the external assets and store them in the contract
        let max_key_uses = drop.max_key_uses;

        let mut total_cost_per_key = 0;
        // Get the total cost and allowance required for a key that has all its uses remaining
        // We'll then multiply this by the number of keys we want to add and charge the user
        get_total_costs_for_key(
            &mut total_cost_per_key,
            max_key_uses,
            &drop.asset_by_id,
            &drop.asset_data_for_uses,
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
        );

        // Write the updated drop data to storage
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure final costs
        let net_storage = env::storage_usage() - initial_storage;
        self.determine_costs(
            key_data.len(),
            false, // No drop was created
            total_cost_per_key,
            net_storage,
            keep_excess_deposit,
        );

        // Now that everything is done (no more potential for panics), we can log the events
        log_events(event_logs);

        true
    }
}
