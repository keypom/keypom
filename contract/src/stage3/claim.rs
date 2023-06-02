use crate::*;

#[near_bindgen]
impl Keypom {
    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: String, password: Option<String>, fc_args: Option<Vec<Option<String>>>) {
        // Delete the access key and remove / return drop data and optional token ID for nft drops. Also return the storage freed.
        let (
            drop_data_option,
            drop_id,
            storage_freed_option,
            token_id,
            should_continue,
            cur_key_id,
            remaining_uses,
            auto_withdraw,
        ) = self.process_claim(password, account_id.clone(), None, &fc_args);

        if drop_data_option.is_none() {
            near_sdk::log!("Invalid claim. Returning.");
            return;
        }

        if should_continue == false {
            near_sdk::log!("Empty function call. Returning.");
            return;
        }

        let drop_data = drop_data_option.unwrap();
        let storage_freed = storage_freed_option.unwrap();

        // Should we refund send back the $NEAR since an account isn't being created and just send the assets to the claiming account?
        let account_to_transfer = if drop_data
            .config
            .as_ref()
            .and_then(|c| c.usage.as_ref())
            .and_then(|u| u.refund_deposit)
            .unwrap_or(false)
            == true
        {
            drop_data.owner_id.clone()
        } else {
            AccountId::new_unchecked(account_id.clone())
        };

        let mut promise = None;
        // Only create a promise to transfer $NEAR if the drop's balance is > 0.
        if drop_data.deposit_per_use > 0 {
            // Send the account ID the desired balance.
            promise = Some(Promise::new(account_to_transfer).transfer(drop_data.deposit_per_use));
        }

        // Execute the callback depending on the drop type. If the drop balance is 0, the promise will be none and the callback function will just straight up be executed instead of resolving the promise.
        self.internal_execute(
            drop_data,
            drop_id.unwrap(),
            cur_key_id,
            remaining_uses,
            AccountId::new_unchecked(account_id),
            storage_freed,
            token_id,
            auto_withdraw,
            fc_args,
            promise
        );

        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "End of regular claim function: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );
    }

    /// Create new account and and claim tokens to it.
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: String,
        new_public_key: String,
        password: Option<String>,
        fc_args: Option<Vec<Option<String>>>,
    ) {
        let (
            drop_data_option,
            drop_id,
            storage_freed_option,
            token_id,
            should_continue,
            cur_key_id,
            remaining_uses,
            auto_withdraw,
        ) = self.process_claim(password, new_account_id.clone(), Some(new_public_key.clone()), &fc_args);

        if drop_data_option.is_none() {
            near_sdk::log!("Invalid claim. Returning.");
            return;
        }

        if should_continue == false {
            near_sdk::log!("Empty function call. Returning.");
            return;
        }

        let drop_data = drop_data_option.unwrap();
        let storage_freed = storage_freed_option.unwrap();
        let global_root = self.root_account.clone();
        let root_account = drop_data
            .config
            .as_ref()
            .and_then(|c| c.root_account_id.as_ref())
            .unwrap_or(&global_root);

        
        // Define the args as strigified JSON
        let mut final_args = format!(
            "{{\"new_account_id\":\"{}\",\"new_public_key\":\"{}\"}}", new_account_id.clone(), new_public_key.clone()
        );

        // If Keypom args are provided in the config, attach them to the create account payload.
        if let Some(keypom_args) = drop_data.config.as_ref().and_then(|c| c.usage.as_ref()).and_then(|u| u.account_creation_fields.as_ref()) {
            final_args = insert_keypom_args_to_ca_payload(final_args, keypom_args.clone(), new_account_id.clone(), drop_id.unwrap().to_string(), cur_key_id.to_string(), drop_data.owner_id.to_string());
        }

        near_sdk::log!("Final payload to create_account: {}", final_args);
        
        // CCC to the linkdrop contract to create the account with the desired balance as the linkdrop amount
        let promise = Promise::new(root_account.clone()).function_call_weight(
            "create_account".to_string(),
            final_args.as_bytes().to_vec(),
                // Attach the balance of the linkdrop along with the exact gas for create account. No unspent GAS is attached.
                drop_data.deposit_per_use,
                GAS_FOR_CREATE_ACCOUNT,
                near_sdk::GasWeight(0),
            );


        // Execute the callback depending on the drop type. We'll pass in the promise to resolve
        self.internal_execute(
            drop_data,
            drop_id.unwrap(),
            cur_key_id,
            remaining_uses,
            AccountId::new_unchecked(new_account_id),
            storage_freed,
            token_id,
            auto_withdraw,
            fc_args,
            Some(promise)
        );

        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "End of on CAAC function: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );
    }

    #[private]
    /// self callback for simple linkdrops with no FTs, NFTs, or FCs.
    pub fn on_claim_simple(
        &mut self,
        // Account ID that sent the funds for the linkdrop
        owner_id: AccountId,
        // Balance contained within the linkdrop
        balance: U128,
        // How much storage was freed when the key was claimed
        storage_used: Balance,
        // Is it an auto withdraw case
        auto_withdraw: bool,
    ) -> bool {
        // Get the status of the cross contract call
        let claim_succeeded = check_promise_result();

        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "Simple on claim used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );

        // Default amount to refund to be the storage used
        let mut amount_to_refund = storage_used;

        near_sdk::log!(
            "Refund Amount (storage used): {}. Auto withdraw: {}",
            yocto_to_near(amount_to_refund),
            auto_withdraw
        );

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            near_sdk::log!(
                "Claim unsuccessful. Refunding linkdrop balance as well: {}",
                balance.0
            );
            amount_to_refund += balance.0
        }

        if auto_withdraw {
            near_sdk::log!(
                "Auto withdraw. Refunding funder: {:?} balance For amount: {:?}",
                owner_id,
                yocto_to_near(amount_to_refund)
            );

            // Send the funds to the funder
            Promise::new(owner_id).transfer(amount_to_refund);
        } else {
            // Get the funder's balance and increment it by the amount to refund
            self.internal_modify_user_balance(&owner_id, amount_to_refund, false);
        }

        claim_succeeded
    }

    #[private]
    /// self callback for FT linkdrop
    pub fn on_claim_ft(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        owner_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was freed when the key was claimed
        storage_used: Balance,
        // FT Data for the drop
        ft_data: FTData,
        // Is it an auto withdraw case
        auto_withdraw: bool,
        // Was this function invoked via an execute (no callback)
        execute: bool,
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();
        near_sdk::log!(
            "Beginning of on claim FT used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );

        // Get the status of the cross contract call. If this function is invoked directly via an execute, default the claim succeeded to true
        let mut claim_succeeded = true;
        if !execute {
            // Get the status of the cross contract call
            claim_succeeded = check_promise_result();
        }
        near_sdk::log!("Has function been executed via CCC: {}", !execute);

        // Default amount to refund to be the storage used
        let mut amount_to_refund = storage_used;

        near_sdk::log!(
            "Refund Amount (storage used): {}. Auto withdraw: {}",
            yocto_to_near(amount_to_refund),
            auto_withdraw
        );

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            near_sdk::log!(
                "Claim unsuccessful. Refunding linkdrop balance as well: {}",
                balance.0
            );
            amount_to_refund += balance.0
        }

        if auto_withdraw {
            near_sdk::log!(
                "Auto withdraw. Refunding funder: {:?} balance For amount: {:?}",
                owner_id,
                yocto_to_near(amount_to_refund)
            );

            // Send the funds to the funder
            Promise::new(owner_id).transfer(amount_to_refund);
        } else {
            // Get the funder's balance and increment it by the amount to refund
            self.internal_modify_user_balance(&owner_id, amount_to_refund, false);
        }

        // Perform the FT transfer functionality
        self.internal_ft_transfer(claim_succeeded, ft_data, account_id);

        claim_succeeded
    }

    #[private]
    /// self callback for a linkdrop loaded with an NFT
    pub fn on_claim_nft(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        owner_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was freed when the key was claimed
        storage_used: Balance,
        // Sender of the NFT
        sender_id: AccountId,
        // Contract where the NFT is stored
        contract_id: AccountId,
        // Token ID for the NFT
        token_id: String,
        // Is it an auto withdraw case
        auto_withdraw: bool,
        // Was this function invoked via an execute (no callback)
        execute: bool,
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "Beginning of on claim NFT used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );

        // Get the status of the cross contract call. If this function is invoked directly via an execute, default the claim succeeded to true
        let mut claim_succeeded = true;
        if !execute {
            // Get the status of the cross contract call
            claim_succeeded = check_promise_result();
        }
        near_sdk::log!("Has function been executed via CCC: {}", !execute);

        // Default amount to refund to be the storage freed
        let mut amount_to_refund = storage_used;

        near_sdk::log!(
            "Refund Amount (storage used): {}. Auto withdraw: {}",
            yocto_to_near(amount_to_refund),
            auto_withdraw
        );

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            near_sdk::log!(
                "Claim unsuccessful. Refunding linkdrop balance as well: {}",
                balance.0
            );
            amount_to_refund += balance.0
        }

        if auto_withdraw {
            near_sdk::log!(
                "Auto withdraw. Refunding funder: {:?} balance For amount: {:?}",
                owner_id,
                yocto_to_near(amount_to_refund)
            );

            // Send the funds to the funder
            Promise::new(owner_id).transfer(amount_to_refund);
        } else {
            // Get the funder's balance and increment it by the amount to refund
            self.internal_modify_user_balance(&owner_id, amount_to_refund, false);
        }

        // Transfer the NFT
        self.internal_nft_transfer(
            claim_succeeded,
            contract_id,
            token_id,
            sender_id,
            account_id,
        );
        claim_succeeded
    }

    #[private]
    /// self callback checks if account was created successfully or not. If yes, refunds excess storage, sends NFTs, FTs etc..
    pub fn on_claim_fc(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        funder_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was freed when the key was claimed
        storage_used: Balance,
        // FC Data for the drop
        fc_data: FCData,
        // Drop ID for the specific drop
        drop_id: DropId,
        // ID for the current key
        cur_key_id: u64,
        // How many uses are remaining on the current key
        remaining_uses: u64,
        // How many uses the key had left before sit was decremented
        uses_per_key: u64,
        // Any user-provided args
        fc_args: Option<Vec<Option<String>>>,
        // Is it an auto withdraw case
        auto_withdraw: bool,
        // Was this function invoked via an execute (no callback)
        execute: bool,
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "Beginning of on claim Function Call used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );

        // Get the status of the cross contract call. If this function is invoked directly via an execute, default the claim succeeded to true
        let mut claim_succeeded = true;
        if !execute {
            // Get the status of the cross contract call
            claim_succeeded = check_promise_result();
        }
        near_sdk::log!("Has function been executed via CCC: {}", !execute);

        // Default amount to refund to be the storage used
        let mut amount_to_refund = storage_used;

        near_sdk::log!(
            "Refund Amount (storage used): {}. Auto withdraw: {}",
            yocto_to_near(amount_to_refund),
            auto_withdraw
        );

        // The starting index is the max uses per key - the number of uses left. If the method_name data is of size 1, use that instead
        let cur_len = fc_data.methods.len() as u16;
        let starting_index = if cur_len > 1 {
            (uses_per_key - remaining_uses) as usize
        } else {
            0 as usize
        };
        let cur_method_data = fc_data
            .methods
            .iter()
            .skip(starting_index)
            .next()
            .unwrap()
            .clone()
            .unwrap();

        // If not successful, the balance and attached_deposit is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            // Get the total_attached deposit from looping through the method datas
            let total_attached_deposit = cur_method_data
                .iter()
                .fold(0, |acc, x| acc + x.attached_deposit.0);

            near_sdk::log!(
                "Claim unsuccessful. Refunding linkdrop balance: {} and attached_deposit: {}",
                balance.0,
                total_attached_deposit
            );
            amount_to_refund += balance.0 + total_attached_deposit
        }

        if auto_withdraw {
            near_sdk::log!(
                "Auto withdraw. Refunding funder: {:?} balance For amount: {:?}",
                funder_id,
                yocto_to_near(amount_to_refund)
            );

            // Send the funds to the funder
            Promise::new(funder_id.clone()).transfer(amount_to_refund);
        } else {
            // Get the funder's balance and increment it by the amount to refund
            self.internal_modify_user_balance(&funder_id, amount_to_refund, false);
        }

        self.internal_fc_execute(
            &cur_method_data,
            cur_key_id,
            account_id,
            funder_id,
            drop_id,
            fc_args
        );
        claim_succeeded
    }

    /// Internal method_name for deleting the used key and removing / returning linkdrop data.
    /// If drop is none, simulate a panic.
    fn process_claim(
        &mut self,
        password: Option<String>,
        account_id: String,
        pub_key: Option<String>,
        fc_args: &Option<Vec<Option<String>>>
    ) -> (
        // Drop containing all data
        Option<Drop>,
        // Drop ID for the drop
        Option<DropId>,
        // How much storage was freed
        Option<Balance>,
        // Next token ID to claim
        Option<String>,
        // Should we return and not do anything once the drop is claimed (if FC data is none)
        bool,
        // ID for the current key
        u64,
        // How many uses are remaining on the current key
        u64,
        // Should we auto withdraw and send the refund to the drop owner's bal
        bool,
    ) {
        let mut used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "Beginning of process claim used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );

        // Pessimistically measure storage
        let initial_storage = env::storage_usage();
        // Ensure only the current contract is calling the method_name using the access key
        // Panic doesn't affect allowance
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );

        // Get the PK of the signer which should be the contract's function call access key
        let signer_pk = env::signer_account_pk();

        // By default, every key should have a drop ID. If we need to remove the key, remove later.
        // Panic doesn't affect allowance
        let token_id = self
            .token_id_by_pk
            .get(&signer_pk)
            .expect("No drop ID found for PK");
        let (drop_id, key_id) = parse_token_id(&token_id);

        // Remove the drop. If the drop shouldn't be removed, we re-insert later.
        // Panic doesn't affect allowance
        let mut drop = self.drop_for_id.remove(&drop_id).expect("drop not found");
        
        // Remove the token ID from the drop's set and check for key usage.
        // Panic doesn't affect allowance
        let mut key_info = drop.key_info_by_token_id.remove(&token_id).unwrap();

        // Keep track of the current number of uses so that it can be used to index into FCData Method Data
        let cur_key_id = key_id;
        let remaining_uses = key_info.remaining_uses;

        // Ensure the account ID and public key are valid types.
        if !self.assert_valid_args(account_id, pub_key, drop_id, &mut drop, &mut key_info, &token_id) {
            return (None, None, None, None, false, 0, 0, false);
        };

        // Ensure the key has enough allowance
        if key_info.allowance < prepaid_gas.0 as u128 * self.yocto_per_gas {
            used_gas = env::used_gas();

            let amount_to_decrement =
                (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
            near_sdk::log!(
                "Not enough allowance on the key {}. Decrementing allowance by {} Used GAS: {}",
                key_info.allowance,
                amount_to_decrement,
                used_gas.0
            );

            key_info.allowance -= amount_to_decrement;
            near_sdk::log!("Allowance is now {}", key_info.allowance);
            drop.key_info_by_token_id.insert(&token_id, &key_info);
            self.drop_for_id.insert(&drop_id, &drop);
            return (None, None, None, None, false, 0, 0, false);
        }

        // If a password was passed in, check it against the key's password
        let cur_use = &(drop
            .config
            .as_ref()
            .and_then(|c| c.uses_per_key)
            .unwrap_or(1)
            - key_info.remaining_uses + 1);

        if self.assert_key_password(
            password,
            drop_id,
            &mut drop,
            &mut key_info,
            cur_use,
            &token_id,
        ) == false
        {
            return (None, None, None, None, false, 0, 0, false);
        };

        // Ensure there's enough uses left for the key to be used. (this *should* only happen in NFT or FT cases)
        if drop.registered_uses < 1 || prepaid_gas != drop.required_gas {
            used_gas = env::used_gas();

            let amount_to_decrement =
                (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
            if drop.registered_uses < 1 {
                near_sdk::log!("Not enough uses left for the drop. Decrementing allowance by {}. Used GAS: {}", amount_to_decrement, used_gas.0);
            } else {
                near_sdk::log!("Prepaid GAS different than what is specified in the drop: {}. Decrementing allowance by {}. Used GAS: {}", drop.required_gas.0, amount_to_decrement, used_gas.0);
            }

            key_info.allowance -= amount_to_decrement;
            near_sdk::log!("Allowance is now {}", key_info.allowance);
            drop.key_info_by_token_id.insert(&token_id, &key_info);
            self.drop_for_id.insert(&drop_id, &drop);
            return (None, None, None, None, false, 0, 0, false);
        }

        if self.assert_claim_timestamps(drop_id, &mut drop, &mut key_info, &token_id) == false {
            return (None, None, None, None, false, 0, 0, false);
        };

        /*
            If it's an NFT drop get the token ID and remove it from the set.
            If it's an FC drop, get the next method_name data and check if it's none (to skip transfer of funds)
        */
        // Default the token ID to none and return / remove the next token ID if it's an NFT drop
        let mut nft_data_token_id = None;
        // Default the should continue variable to true. If the next FC method_name is None, we set it to false
        let mut should_continue = true;
        match &mut drop.drop_type {
            DropType::nft(data) => {
                nft_data_token_id = data.token_ids.pop();
            }
            DropType::fc(data) => {
                let starting_index = if data.methods.len() > 1 {
                    (drop
                        .config
                        .as_ref()
                        .and_then(|c| c.uses_per_key)
                        .unwrap_or(1)
                        - key_info.remaining_uses) as usize
                } else {
                    0 as usize
                };

                let cur_parallel_methods = data
                    .methods
                    .iter()
                    .skip(starting_index)
                    .next()
                    .unwrap()
                    .clone()
                    .and_then(|m| Some(m.len()))
                    .unwrap_or(0);

                should_continue = cur_parallel_methods.clone() > 0 as usize;

                if should_continue {
                    // Ensure that if the FC args are passed in, it's the same length as the number of methods being executed.
                    if fc_args.as_ref().map(|a| a.len()).unwrap_or(cur_parallel_methods) != cur_parallel_methods {
                        used_gas = env::used_gas();
                        let amount_to_decrement =
                            (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;

                        near_sdk::log!("Expected {} number of fc_args. Decrementing allowance by {}. Used GAS: {}", cur_parallel_methods, amount_to_decrement, used_gas.0);
                        key_info.allowance -= amount_to_decrement;
                        near_sdk::log!("Allowance is now {}", key_info.allowance);
                        drop.key_info_by_token_id.insert(&token_id, &key_info);
                        self.drop_for_id.insert(&drop_id, &drop);
                        return (None, None, None, None, false, 0, 0, false);
                    }
                }
            }
            _ => {}
        };

        // Default whether or not the key should be deleted as true
        let mut should_delete_key = true;
        near_sdk::log!(
            "Key usage last used: {:?} Num uses: {:?} (before)",
            key_info.last_used,
            key_info.remaining_uses
        );

        // No uses left! The key should be deleted
        if key_info.remaining_uses == 1 {
            near_sdk::log!("Key has no uses left. It will be deleted");
            self.token_id_by_pk.remove(&signer_pk);
            if let Some(owner) = key_info.owner_id {
                self.internal_remove_token_from_owner(&owner, &token_id);
            }
        } else {
            key_info.remaining_uses -= 1;
            key_info.allowance -= drop.required_gas.0 as u128 * self.yocto_per_gas;
            near_sdk::log!(
                "Key has {} uses left. Decrementing allowance by {}. Allowance left: {}",
                key_info.remaining_uses,
                drop.required_gas.0 as u128 * self.yocto_per_gas,
                key_info.allowance
            );

            drop.key_info_by_token_id.insert(&token_id, &key_info);
            should_delete_key = false;
        }

        drop.registered_uses -= 1;

        // If there are keys still left in the drop, add the drop back in with updated data
        if !drop.key_info_by_token_id.is_empty() {
            // Add drop back with the updated data.
            self.drop_for_id.insert(&drop_id, &drop);
        } else {
            // There are no keys left. We should only remove the drop if the drop's config is set to delete on empty
            if drop
                .config
                .as_ref()
                .and_then(|c| c.usage.as_ref())
                .and_then(|u| u.auto_delete_drop)
                .unwrap_or(false)
            {
                near_sdk::log!("Drop is empty and delete_on_empty is set to true. Deleting drop");
                // Remove the drop ID from the funder's list if the drop is now empty
                self.internal_remove_drop_for_funder(&drop.owner_id, &drop_id);
            } else {
                near_sdk::log!("Drop is empty but delete_on_empty is not specified. Keeping drop");
                // Add drop back with the updated data.
                self.drop_for_id.insert(&drop_id, &drop);
            }
        }

        let mut should_auto_withdraw = false;
        if should_delete_key {
            // Amount to refund is the current allowance less the current execution's max GAS
            let mut amount_to_refund = ACCESS_KEY_STORAGE;
            // Only refund unspent allowance if the funder is in the allowlist
            if self.refund_allowlist.contains(&drop.owner_id) {
                amount_to_refund += key_info.allowance
                - drop.required_gas.0 as u128 * self.yocto_per_gas;
                near_sdk::log!(
                    "Funder is on allowlist. Adding to refund amount.
                    Allowance Currently: {}. 
                    Drop required gas: {}",
                    key_info.allowance,
                    drop.required_gas.0 as u128 * self.yocto_per_gas,
                );
            }

            near_sdk::log!(
                "Key being deleted. Will refund: {}",
                amount_to_refund,
            );

            // Check if auto_withdrawing to the funder's entire balance
            let auto_withdraw = drop
                .config
                .as_ref()
                .and_then(|c| c.usage.as_ref())
                .and_then(|u| u.auto_withdraw)
                .unwrap_or(false);

            // Get the number of drops still left for the owner
            let cur_drop_num_for_owner = self
                .drop_ids_for_funder
                .get(&drop.owner_id)
                .and_then(|d| Some(d.len()))
                .unwrap_or(0);

            // If auto_withdraw is set to true and this is the last drop for the owner, we should just withdraw the entire balance
            if auto_withdraw && cur_drop_num_for_owner == 0 {
                should_auto_withdraw = true;
                let mut cur_balance = self.user_balances.remove(&drop.owner_id).unwrap_or(0);
                cur_balance += amount_to_refund;
                near_sdk::log!(
                    "Auto withdrawing the entire balance of {}.",
                    yocto_to_near(cur_balance)
                );

                // Send cur balance to drop owner
                Promise::new(drop.owner_id.clone()).transfer(cur_balance);
            } else {
                // Get the funder's balance and increment it by the amount to refund
                self.internal_modify_user_balance(&drop.owner_id, amount_to_refund, false);
            }

            // Delete the key
            Promise::new(env::current_account_id()).delete_key(signer_pk);
        }

        // Calculate the storage being freed. initial - final should be >= 0 since final should be smaller than initial.
        let final_storage = env::storage_usage();
        let total_storage_freed =
            Balance::from(initial_storage - final_storage) * env::storage_byte_cost();
        near_sdk::log!(
            "Total storage freed: {}. Initial storage: {}. Final storage: {}",
            total_storage_freed,
            initial_storage,
            final_storage
        );

        // Return the drop and optional token ID with how much storage was freed
        (
            Some(drop),
            Some(drop_id),
            Some(total_storage_freed),
            nft_data_token_id,
            should_continue,
            cur_key_id,
            remaining_uses,
            should_auto_withdraw,
        )
    }
}
