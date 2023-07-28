use crate::*;

#[near_bindgen]
impl Keypom {
    #[payable]
    /// Allows a user to add a list of account IDs to the sale's allowlist
    pub fn add_to_sale_allowlist(
        &mut self,
        drop_id: DropId,
        account_ids: Vec<AccountId>
    ) {
        self.asset_no_global_freeze();

        let initial_storage = env::storage_usage();
        
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_by_id.get(&drop_id).expect("no drop found");
        let caller_id = env::predecessor_account_id();
        require!(caller_id == drop.funder_id, "only drop funder can update sale config");
        
        let mut config = drop.drop_config.expect("no config found");
        let mut sale = config.sale.expect("no sale found for drop");
        let mut actual_allowlist = sale.allowlist.unwrap_or(Default::default());

        // Loop through and add all the accounts to the allow list
        for account in account_ids {
            actual_allowlist.insert(account);
        }
        
        sale.allowlist = Some(actual_allowlist);
        config.sale = Some(sale);
        drop.drop_config = Some(config);
        self.drop_by_id.insert(&drop_id, &drop);

        // Calculate the storage being used
        let final_storage = env::storage_usage();
        let total_required_storage =
            Balance::from(final_storage - initial_storage) * env::storage_byte_cost();

        self.charge_with_deposit_or_balance(total_required_storage);
    }

    /// Allows a user to remove a list of account IDs from the sale's allowlist
    pub fn remove_from_sale_allowlist(
        &mut self,
        drop_id: DropId,
        account_ids: Vec<AccountId>
    ) {
        self.asset_no_global_freeze();

        let initial_storage = env::storage_usage();
        
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_by_id.get(&drop_id).expect("no drop found");
        let caller_id = env::predecessor_account_id();
        require!(caller_id == drop.funder_id, "only drop funder can update sale config");
        
        let mut config = drop.drop_config.expect("no config found");
        let mut sale = config.sale.expect("no sale found for drop");
        let mut actual_allowlist = sale.allowlist.unwrap_or(Default::default());

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
        drop.drop_config = Some(config);
        self.drop_by_id.insert(&drop_id, &drop);

        // Calculate the storage being freed
        let final_storage = env::storage_usage();
        let total_storage_freed =
            Balance::from(initial_storage - final_storage) * env::storage_byte_cost();

        self.internal_modify_user_balance(&caller_id, total_storage_freed, false);
    }

    /// Allows a user to add a list of account IDs to the sale's blocklist
    pub fn add_to_sale_blocklist(
        &mut self,
        drop_id: DropId,
        account_ids: Vec<AccountId>
    ) {
        self.asset_no_global_freeze();

        let initial_storage = env::storage_usage();
        
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_by_id.get(&drop_id).expect("no drop found");
        let caller_id = env::predecessor_account_id();
        require!(caller_id == drop.funder_id, "only drop funder can update sale config");
        
        let mut config = drop.drop_config.expect("no config found");
        let mut sale = config.sale.expect("no sale found for drop");
        let mut actual_blocklist = sale.blocklist.unwrap_or(Default::default());

        // Loop through and add all the accounts to the allow list
        for account in account_ids {
            actual_blocklist.insert(account);
        }
        
        sale.blocklist = Some(actual_blocklist);
        config.sale = Some(sale);
        drop.drop_config = Some(config);
        self.drop_by_id.insert(&drop_id, &drop);

        // Calculate the storage being used
        let final_storage = env::storage_usage();
        let total_required_storage =
            Balance::from(final_storage - initial_storage) * env::storage_byte_cost();

        self.charge_with_deposit_or_balance(total_required_storage);
    }

    /// Allows a user to remove a list of account IDs from the sale's blocklist
    pub fn remove_from_sale_blocklist(
        &mut self,
        drop_id: DropId,
        account_ids: Vec<AccountId>
    ) {
        self.asset_no_global_freeze();

        let initial_storage = env::storage_usage();
        
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_by_id.get(&drop_id).expect("no drop found");
        let caller_id = env::predecessor_account_id();
        require!(caller_id == drop.funder_id, "only drop funder can update sale config");
        
        let mut config = drop.drop_config.expect("no config found");
        let mut sale = config.sale.expect("no sale found for drop");
        let mut actual_blocklist = sale.blocklist.unwrap_or(Default::default());

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
        drop.drop_config = Some(config);
        self.drop_by_id.insert(&drop_id, &drop);

        // Calculate the storage being freed
        let final_storage = env::storage_usage();
        let total_storage_freed =
            Balance::from(initial_storage - final_storage) * env::storage_byte_cost();

        self.internal_modify_user_balance(&caller_id, total_storage_freed, false);
    }

    /// Overwrite the passed in sale configurations for a given drop ID. This method will panic if the sale doesn't exist.
    #[payable]
    pub fn update_sale(
        &mut self,
        drop_id: DropId,
        max_num_keys: Option<u64>,
        price_per_key: Option<U128>,
        start: Option<u64>,
        end: Option<u64>,
    ) {
        self.asset_no_global_freeze();
        
        let initial_storage = env::storage_usage();
        
        require!(max_num_keys.is_some() || price_per_key.is_some() || start.is_some() || end.is_some(), "no parameters provided");

        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_by_id.get(&drop_id).expect("no drop found");
        let caller_id = env::predecessor_account_id();
        require!(caller_id == drop.funder_id, "only drop funder can update sale config");
        
        let mut config = drop.drop_config.expect("no config found");
        let sale = config.sale.expect("no sale found for drop");

        config.sale = Some(PublicSaleConfig { 
            max_num_keys: max_num_keys.or(sale.max_num_keys),
            price_per_key: price_per_key.map(|p| p.0).or(sale.price_per_key),
            allowlist: sale.allowlist,
            blocklist: sale.blocklist,
            start: start.or(sale.start),
            end: end.or(sale.end),
        });
        drop.drop_config = Some(config);
        self.drop_by_id.insert(&drop_id, &drop);

        let final_storage = env::storage_usage();
        // We freed storage and the user should be refunded
        if initial_storage > final_storage {
            let total_storage_freed =
            Balance::from(initial_storage - final_storage) * env::storage_byte_cost();

            self.internal_modify_user_balance(&caller_id, total_storage_freed, false);
        } else {
            let total_required_storage =
            Balance::from(final_storage - initial_storage) * env::storage_byte_cost();

            self.charge_with_deposit_or_balance(total_required_storage); 
        }
    }

    /// Check if the timestamps and allowlists are fulfilled.
    /// If everything is fulfilled, return the total $NEAR to be charged to the user
    /// And also add to the funder's user balance
    pub(crate) fn assert_sale_requirements(
        &mut self,
        funder_id: &AccountId,
        sale: &PublicSaleConfig, 
        cur_num_keys: u64, 
        num_keys_to_add: u64
    ) -> u128 {
        // Assert that the current time is between the start and end time
        let cur_time = env::block_timestamp();
        let desired_start = sale.start.unwrap_or(0);
        let desired_end = sale.end.unwrap_or(u64::MAX);
        require!(
            cur_time >= desired_start && cur_time <= desired_end,
            "Public Sale Has Ended"
        );

        // Assert that the current number of keys is less than the max number of keys
        let max_num_keys = sale.max_num_keys.unwrap_or(u64::MAX);
        require!(
            cur_num_keys + num_keys_to_add <= max_num_keys,
            "Cannot add more keys than the max number of keys"
        );

        // Assert that the current account is in the allow list
        if let Some(list) = &sale.allowlist {
            assert!(
                list.contains(&env::predecessor_account_id()),
                "Only members in the allowlist can add keys"
            );
        }

        // Assert that the current account is not in the block list
        if let Some(list) = &sale.blocklist {
            assert!(
                !list.contains(&env::predecessor_account_id()),
                "Only members not in the blocklist can add keys"
            );
        }

        // Send the $NEAR to the funder
        let revenue_generated = sale.price_per_key.unwrap_or(0) * num_keys_to_add as u128;
        self.internal_modify_user_balance(funder_id, revenue_generated, false);

        // Return the $NEAR
        revenue_generated
    }

}