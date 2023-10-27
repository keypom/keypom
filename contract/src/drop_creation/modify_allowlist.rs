use std::collections::HashSet;

use crate::*;

#[near_bindgen]
impl Keypom {
    /// Allows a user to add a list of account IDs to the sale's allowlist
    #[payable]
    pub fn add_to_sale_allowlist(
        &mut self,
        drop_id: DropId,
        account_ids: Vec<AccountId>
    ) {
        self.assert_no_global_freeze();

        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);

        // get the drop object (remove it and only re-insert at the end if it shouldn't be deleted)
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");
        
        let funder_id = drop.funder_id.clone();
        let caller_id = env::predecessor_account_id();

        require!(caller_id == funder_id, "Only drop funder can add accounts to allowlist");


        // If there is an allowlist, append to existing one. Otherwise, create a new one. 
        if let Some(allowlist) = drop.config.as_mut().and_then(|c| c.add_key_allowlist.as_mut()) {
            for account in account_ids {
                if !allowlist.contains(&account) {
                    near_sdk::log!("existing allowlist");
                    allowlist.insert(account);
                }
            }
        } else {
            let mut allowlist = HashSet::new();
            for account in account_ids {
                near_sdk::log!("no existing allowlist");
                allowlist.insert(account);
            }
            near_sdk::log!("Allowlist to be freshly added: {:?}", allowlist);

            if let Some(config) = &mut drop.config {
                config.add_key_allowlist = Some(allowlist);
            } else {
                // If drop.config doesn't exist, create it with add_key_allowlist
                drop.config = Some(DropConfig { 
                    metadata: None, 
                    nft_keys_config: None, 
                    add_key_allowlist: Some(allowlist),
                    delete_empty_drop: None, 
                    extra_allowance_per_key: None,
                });
            }
        }

        // Write the updated drop data to storage
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure final costs and charge user
        let net_storage = env::storage_usage() - initial_storage;
        self.determine_costs(
            0,
            false, // No drop was created
            0,
            0,
            net_storage,
            None
        );

        // test this, i doubt it will work
        
    }

    //#[payable]
    /// Allows a user to remove a list of account IDs from the sale's allowlist
    pub fn remove_from_sale_allowlist(
        &mut self,
        drop_id: DropId,
        account_ids: Vec<AccountId>
    ) {
        self.assert_no_global_freeze();

        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);

        // get the drop object (remove it and only re-insert at the end if it shouldn't be deleted)
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");
        
        let funder_id = drop.funder_id.clone();
        let caller_id = env::predecessor_account_id();

        require!(caller_id == funder_id, "Only drop funder can remove accounts to allowlist");

        // If there is an allowlist, append to existing one. Otherwise, create a new one. 
        if let Some(allowlist) = drop.config.as_mut().and_then(|c| c.add_key_allowlist.as_mut()) {
            for account in account_ids {
                if allowlist.contains(&account) {
                    //near_sdk::log!("Found account: {}", account);
                    allowlist.remove(&account);
                }
            }
        } else {
            near_sdk::log!("Could not remove accounts from allowlist: Drop has no allowlist");
        }

        // Write the updated drop data to storage
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure final costs and charge user
        // let net_storage = env::storage_usage() - initial_storage;
        // self.determine_costs(
        //     0,
        //     false, // No drop was created
        //     0,
        //     0,
        //     net_storage,
        //     None
        // );
    }
}