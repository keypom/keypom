use crate::*;

#[near_bindgen]
impl Keypom {
    #[payable]
    /// Allows a user to add a list of account IDs to the sale's allowlist
    pub fn add_to_sale_allowlist(
        &mut self,
        drop_id: DropIdJson,
        account_ids: Vec<AccountId>
    ) {
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        let owner_id = env::predecessor_account_id();
        require!(owner_id == drop.owner_id, "only drop owner can update sale config");
        
        let mut config = drop.config.expect("no config found");
        let initial_storage = env::storage_usage();
        let (current_user_balance, _) = self.attached_deposit_to_user_balance(&owner_id);

        if let Some(mut sale) = config.sale {
            let mut actual_allowlist = sale.allowlist.unwrap_or(UnorderedSet::new(StorageKey::PubSaleAllowlist {
                //we get a new unique prefix for the collection
                account_id_hash: hash_account_id(&format!("allowlist-{}{}", drop_id.0, drop.owner_id)),
            }));

            // Loop through and add all the accounts to the allow list
            for account in account_ids {
                actual_allowlist.insert(&account);
            }
            
            sale.allowlist = Some(actual_allowlist);
            config.sale = Some(sale);
            drop.config = Some(config);
            self.drop_for_id.insert(&drop_id.0, &drop);
        } else {
            env::panic_str("no sale found for drop");
        }

        // Calculate the storage being used
        let final_storage = env::storage_usage();
        let total_required_storage =
            Balance::from(final_storage - initial_storage) * env::storage_byte_cost();
        near_sdk::log!("Total required storage Yocto {}", total_required_storage);
        require!(current_user_balance >= total_required_storage, "not enough balance to pay for storage");
        self.internal_modify_user_balance(&owner_id, total_required_storage, true); 
    }

    /// Allows a user to remove a list of account IDs from the sale's allowlist
    pub fn remove_from_sale_allowlist(
        &mut self,
        drop_id: DropIdJson,
        account_ids: Vec<AccountId>
    ) {
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        let owner_id = env::predecessor_account_id();
        require!(owner_id == drop.owner_id, "only drop owner can update sale config");
        
        let mut config = drop.config.expect("no config found");
        let initial_storage = env::storage_usage();

        if let Some(mut sale) = config.sale {
            let mut actual_allowlist = sale.allowlist.expect("no allowlist found");

            // Loop through and remove all the accounts to the allow list
            for account in account_ids {
                actual_allowlist.remove(&account);
            }

            if actual_allowlist.is_empty() {
                sale.allowlist = None;
            } else {
                sale.allowlist = Some(actual_allowlist);
            }
            
            config.sale = Some(sale);
            drop.config = Some(config);
            self.drop_for_id.insert(&drop_id.0, &drop);
        } else {
            env::panic_str("no sale found for drop");
        }

        // Calculate the storage being freed
        let final_storage = env::storage_usage();
        let total_storage_freed =
            Balance::from(initial_storage - final_storage) * env::storage_byte_cost();
        near_sdk::log!("Total required freed Yocto {}", total_storage_freed);
        self.internal_modify_user_balance(&owner_id, total_storage_freed, false); 
    }

    /// Allows a user to add a list of account IDs to the sale's blocklist
    pub fn add_to_sale_blocklist(
        &mut self,
        drop_id: DropIdJson,
        account_ids: Vec<AccountId>
    ) {
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        let owner_id = env::predecessor_account_id();
        require!(owner_id == drop.owner_id, "only drop owner can update sale config");
        
        let mut config = drop.config.expect("no config found");
        let initial_storage = env::storage_usage();
        let (current_user_balance, _) = self.attached_deposit_to_user_balance(&owner_id);

        if let Some(mut sale) = config.sale {
            let mut actual_blocklist = sale.blocklist.unwrap_or(UnorderedSet::new(StorageKey::PubSaleBlocklist {
                //we get a new unique prefix for the collection
                account_id_hash: hash_account_id(&format!("blocklist-{}{}", drop_id.0, drop.owner_id)),
            }));

            // Loop through and add all the accounts to the block list
            for account in account_ids {
                actual_blocklist.insert(&account);
            }
            
            sale.blocklist = Some(actual_blocklist);
            config.sale = Some(sale);
            drop.config = Some(config);
            self.drop_for_id.insert(&drop_id.0, &drop);
        } else {
            env::panic_str("no sale found for drop");
        }

        // Calculate the storage being used
        let final_storage = env::storage_usage();
        let total_required_storage =
            Balance::from(final_storage - initial_storage) * env::storage_byte_cost();
        near_sdk::log!("Total required storage Yocto {}", total_required_storage);
        require!(current_user_balance >= total_required_storage, "not enough balance to pay for storage");
        self.internal_modify_user_balance(&owner_id, total_required_storage, true); 
    }

    /// Allows a user to remove a list of account IDs from the sale's blocklist
    pub fn remove_from_sale_blocklist(
        &mut self,
        drop_id: DropIdJson,
        account_ids: Vec<AccountId>
    ) {
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        let owner_id = env::predecessor_account_id();
        require!(owner_id == drop.owner_id, "only drop owner can update sale config");
        
        let mut config = drop.config.expect("no config found");
        let initial_storage = env::storage_usage();

        if let Some(mut sale) = config.sale {
            let mut actual_blocklist = sale.blocklist.expect("no blocklist found");

            // Loop through and remove all the accounts from the blocklist
            for account in account_ids {
                actual_blocklist.remove(&account);
            }

            if actual_blocklist.is_empty() {
                sale.blocklist = None;
            } else {
                sale.blocklist = Some(actual_blocklist);
            }

            config.sale = Some(sale);
            drop.config = Some(config);
            self.drop_for_id.insert(&drop_id.0, &drop);
        } else {
            env::panic_str("no sale found for drop");
        }

        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_storage_freed =
            Balance::from(initial_storage - final_storage) * env::storage_byte_cost();
        self.internal_modify_user_balance(&owner_id, total_storage_freed, false); 
    }

    /// Overwrite the passed in sale configurations for a given drop ID. This method will panic if the sale doesn't exist.
    #[payable]
    pub fn update_sale(
        &mut self,
        drop_id: DropIdJson,
        max_num_keys: Option<u64>,
        price_per_key: Option<U128>,
        auto_withdraw_funds: Option<bool>,
        start: Option<u64>,
        end: Option<u64>,
    ) {
        require!(max_num_keys.is_some() || price_per_key.is_some() || auto_withdraw_funds.is_some() || start.is_some() || end.is_some(), "no parameters provided");

        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        let owner_id = env::predecessor_account_id();
        require!(owner_id == drop.owner_id, "only drop owner can update sale config");
        
        let mut config = drop.config.expect("no config found");
        let initial_storage = env::storage_usage();
        let (current_user_balance, _) = self.attached_deposit_to_user_balance(&owner_id);

        if let Some(mut sale) = config.sale {
            sale.max_num_keys = max_num_keys;
            sale.price_per_key = price_per_key.map(|p| p.0);
            sale.auto_withdraw_funds = auto_withdraw_funds;
            sale.start = start;
            sale.end = end;

            config.sale = Some(sale);
            drop.config = Some(config);
            self.drop_for_id.insert(&drop_id.0, &drop);
        } else {
            env::panic_str("no sale found for drop");
        }

        // Calculate the storage being freed
        let final_storage = env::storage_usage();

        // We freed storage and the user should be refunded
        if initial_storage > final_storage {
            let total_storage_freed =
                Balance::from(initial_storage - final_storage) * env::storage_byte_cost();
            near_sdk::log!("Total storage freed Yocto {}", total_storage_freed);
            self.internal_modify_user_balance(&owner_id, total_storage_freed, false); 
        } else {
            let total_required_storage =
                Balance::from(final_storage - initial_storage) * env::storage_byte_cost();
            near_sdk::log!("Total required storage Yocto {}", total_required_storage);
            require!(current_user_balance >= total_required_storage, "not enough balance to pay for storage");
            self.internal_modify_user_balance(&owner_id, total_required_storage, true); 
        }
    }
}