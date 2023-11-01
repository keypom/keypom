use std::collections::HashSet;

use crate::*;

#[near_bindgen]
impl Keypom {
    /// Adds a list of account IDs to the sale's allowlist for a specific drop.
    /// This function can only be called by the drop funder.
    #[payable]
    pub fn add_to_sale_allowlist(
        &mut self,
        drop_id: DropId,
        account_ids: Vec<AccountId>
    ) {
        // Ensure that the global freeze is not active.
        self.assert_no_global_freeze();

        // Measure the initial storage usage for cost calculation.
        let initial_storage = env::storage_usage();

        // Retrieve the drop object by its ID or panic if not found.
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");

        // Clone the funder's account ID and get the caller's account ID.
        let funder_id = drop.funder_id.clone();
        let caller_id = env::predecessor_account_id();

        // Ensure that only the drop funder can add accounts to the allowlist.
        require!(caller_id == funder_id, "Only drop funder can add accounts to allowlist");

        // Get a mutable reference to the drop configuration, creating a new one if it doesn't exist.
        let mut config = drop.config.unwrap_or(DropConfig {
            metadata: None,
            nft_keys_config: None,
            add_key_allowlist: Some(HashSet::new()),
            delete_empty_drop: None,
            extra_allowance_per_key: None,
        });

        let mut allowlist = config.add_key_allowlist.unwrap_or(HashSet::new());
        // Insert the provided account IDs into the allowlist.
        for account in account_ids {
            allowlist.insert(account);
        }

        // Write the updated drop data back to storage.
        config.add_key_allowlist = Some(allowlist);
        drop.config = Some(config);
        self.drop_by_id.insert(&drop_id, &drop);

        // Calculate the net storage usage and charge the user accordingly.
        let net_storage = env::storage_usage() - initial_storage;
        let total_cost = net_storage as u128 * env::storage_byte_cost();
        let keep_excess_costs = true;
        self.charge_with_deposit_or_balance(total_cost, Some(keep_excess_costs));
    }

    /// Allows a user to remove a list of account IDs from the sale's allowlist for a specific drop.
    pub fn remove_from_sale_allowlist(
        &mut self,
        drop_id: DropId,
        account_ids: Vec<AccountId>
    ) {
        // Ensure that the global freeze is not active.
        self.assert_no_global_freeze();

        // Measure the initial storage usage for cost calculation.
        let initial_storage = env::storage_usage();

        // Get the drop object (remove it and only re-insert at the end if it shouldn't be deleted).
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");
        
        let funder_id = drop.funder_id.clone();
        let caller_id = env::predecessor_account_id();

        // Ensure that only the drop funder can remove accounts from the allowlist.
        require!(caller_id == funder_id, "Only drop funder can remove accounts from allowlist");

        // If there is an allowlist, remove accounts, otherwise panic
        let allowlist = drop.config.as_mut().and_then(|c| c.add_key_allowlist.as_mut()).expect("No allowlist found");
        for account in account_ids {
            allowlist.remove(&account);
        }
        
        // Write the updated drop data to storage.
        self.drop_by_id.insert(&drop_id, &drop);

        // Refund user?
        let storage_released = initial_storage - env::storage_usage();
        let refund_amount = storage_released as u128 * env::storage_byte_cost();
        let increment_balance = true;

        self.internal_modify_user_balance(&caller_id, refund_amount, increment_balance);
    }
}