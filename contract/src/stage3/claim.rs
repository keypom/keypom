use crate::*;

#[near_bindgen]
impl DropZone {
    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) {
        // Delete the access key and remove / return drop data and optional token ID for nft drops. Also return the storage freed.
        let (drop_data_option, drop_id, storage_freed_option, token_id, storage_for_longest) =
            self.process_claim();

        if drop_data_option.is_none() {
            near_sdk::log!("Invalid claim. Returning.");
            return;
        }
        let drop_data = drop_data_option.unwrap();
        let storage_freed = storage_freed_option.unwrap();

        // Should we refund send back the $NEAR since an account isn't being created and just send the assets to the claiming account?
        let account_to_transfer = if drop_data.drop_config.refund_if_claim.unwrap_or(false) == true
        {
            drop_data.funder_id.clone()
        } else {
            account_id.clone()
        };

        let mut promise = None;
        // Only create a promise to transfer $NEAR if the drop's balance is > 0.
        if drop_data.balance.0 > 0 {
            // Send the account ID the desired balance.
            promise = Some(Promise::new(account_to_transfer).transfer(drop_data.balance.0));
        }

        // Execute the callback depending on the drop type. If the drop balance is 0, the promise will be none and the callback function will just straight up be executed instead of resolving the promise.
        self.internal_execute(
            drop_data,
            drop_id.unwrap(),
            account_id,
            storage_freed,
            token_id,
            storage_for_longest,
            promise,
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
        new_account_id: AccountId,
        new_public_key: PublicKey,
    ) {
        let (drop_data_option, drop_id, storage_freed_option, token_id, storage_for_longest) =
            self.process_claim();

        if drop_data_option.is_none() {
            near_sdk::log!("Invalid claim. Returning.");
            return;
        }
        let drop_data = drop_data_option.unwrap();
        let storage_freed = storage_freed_option.unwrap();

        // CCC to the linkdrop contract to create the account with the desired balance as the linkdrop amount
        let promise = ext_linkdrop::ext(self.linkdrop_contract.clone())
            // Attach the balance of the linkdrop along with the exact gas for create account. No unspent GAS is attached.
            .with_attached_deposit(drop_data.balance.0)
            .with_static_gas(GAS_FOR_CREATE_ACCOUNT)
            .with_unused_gas_weight(0)
            .create_account(new_account_id.clone(), new_public_key);

        // Execute the callback depending on the drop type. We'll pass in the promise to resolve
        self.internal_execute(
            drop_data,
            drop_id.unwrap(),
            new_account_id,
            storage_freed,
            token_id,
            storage_for_longest,
            Some(promise),
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
        funder_id: AccountId,
        // Balance contained within the linkdrop
        balance: U128,
        // How much storage was freed when the key was claimed
        storage_used: Balance,
    ) -> bool {
        // Get the status of the cross contract call
        let claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "Simple on claim used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund = ACCESS_KEY_STORAGE + storage_used;

        near_sdk::log!(
            "Refund Amount: {}, 
            Access Key Storage: {}, 
            Storage Used: {}",
            yocto_to_near(amount_to_refund),
            yocto_to_near(ACCESS_KEY_STORAGE),
            yocto_to_near(storage_used)
        );

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            near_sdk::log!(
                "Claim unsuccessful. Refunding linkdrop balance as well: {}",
                balance.0
            );
            amount_to_refund += balance.0
        }

        near_sdk::log!(
            "Refunding funder: {:?} For amount: {:?}",
            funder_id,
            yocto_to_near(amount_to_refund)
        );

        // Get the funder's balance and increment it by the amount to refund
        let mut cur_funder_balance = self
            .user_balances
            .get(&funder_id)
            .expect("No funder balance found");
        cur_funder_balance += amount_to_refund;
        self.user_balances.insert(&funder_id, &cur_funder_balance);

        claim_succeeded
    }

    #[private]
    /// self callback for FT linkdrop
    pub fn on_claim_ft(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        funder_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was freed when the key was claimed
        storage_used: Balance,
        // FT Data for the drop
        ft_data: FTData,
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
            claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));
        }
        near_sdk::log!("Has function been executed via CCC: {}", !execute);

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund = ACCESS_KEY_STORAGE + storage_used;

        near_sdk::log!(
            "Refund Amount: {}, 
            Access Key Storage: {}, 
            Storage Used: {}",
            yocto_to_near(amount_to_refund),
            yocto_to_near(ACCESS_KEY_STORAGE),
            yocto_to_near(storage_used)
        );

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            near_sdk::log!(
                "Claim unsuccessful. Refunding linkdrop balance as well: {}",
                balance.0
            );
            amount_to_refund += balance.0
        }

        near_sdk::log!(
            "Refunding funder: {:?} balance For amount: {:?}",
            funder_id,
            yocto_to_near(amount_to_refund)
        );
        // Get the funder's balance and increment it by the amount to refund
        let mut cur_funder_balance = self
            .user_balances
            .get(&funder_id)
            .expect("No funder balance found");
        cur_funder_balance += amount_to_refund;
        self.user_balances.insert(&funder_id, &cur_funder_balance);

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
        funder_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was freed when the key was claimed
        storage_used: Balance,
        // How much storage was prepaid to cover the longest token ID being inserted.
        storage_for_longest: Balance,
        // Sender of the NFT
        nft_sender: AccountId,
        // Contract where the NFT is stored
        nft_contract: AccountId,
        // Token ID for the NFT
        token_id: String,
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
            claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));
        }
        near_sdk::log!("Has function been executed via CCC: {}", !execute);

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        // In addition, we refund them for the cost of storing the longest token ID now that a key has been claimed
        let mut amount_to_refund =
            ACCESS_KEY_STORAGE + storage_used + storage_for_longest * env::storage_byte_cost();

        near_sdk::log!(
            "Refund Amount: {}, 
            Access Key Storage: {}, 
            Storage Used: {}
            Storage for longest: {}",
            yocto_to_near(amount_to_refund),
            yocto_to_near(ACCESS_KEY_STORAGE),
            yocto_to_near(storage_used),
            yocto_to_near(storage_for_longest * env::storage_byte_cost())
        );

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            near_sdk::log!(
                "Claim unsuccessful. Refunding linkdrop balance as well: {}",
                balance.0
            );
            amount_to_refund += balance.0
        }

        near_sdk::log!(
            "Refunding funder: {:?} balance For amount: {:?}",
            funder_id,
            yocto_to_near(amount_to_refund)
        );
        // Get the funder's balance and increment it by the amount to refund
        let mut cur_funder_balance = self
            .user_balances
            .get(&funder_id)
            .expect("No funder balance found");
        cur_funder_balance += amount_to_refund;
        self.user_balances.insert(&funder_id, &cur_funder_balance);

        // Transfer the NFT
        self.internal_nft_transfer(
            claim_succeeded,
            nft_contract,
            token_id,
            nft_sender,
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
            claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));
        }
        near_sdk::log!("Has function been executed via CCC: {}", !execute);

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund = ACCESS_KEY_STORAGE + storage_used;

        near_sdk::log!(
            "Refund Amount: {}, 
            Access Key Storage: {}, 
            Storage Used: {}",
            yocto_to_near(amount_to_refund),
            yocto_to_near(ACCESS_KEY_STORAGE),
            yocto_to_near(storage_used)
        );

        // If not successful, the balance and deposit is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            near_sdk::log!(
                "Claim unsuccessful. Refunding linkdrop balance: {} and deposit: {}",
                balance.0,
                fc_data.deposit.0
            );
            amount_to_refund += balance.0 + fc_data.deposit.0
        }

        /*
            If the claim is not successful, we should always refund. The only case where we don't refund is
            if the claim was successful and the user specified that the refund should go into the
            deposit.

            0 0     Refund     !success  -> do refund
            0 1     Refund      success  -> do refund
            1 0     No Refund  !success  -> do refund
            1 1     No Refund   Success  -> don't do refund
        */
        if !claim_succeeded || (!fc_data.refund_to_deposit.unwrap_or(false) && claim_succeeded) {
            // Refunding
            near_sdk::log!(
                "Refunding funder: {:?} balance For amount: {:?}",
                funder_id,
                yocto_to_near(amount_to_refund)
            );
            // Get the funder's balance and increment it by the amount to refund
            let mut cur_funder_balance = self
                .user_balances
                .get(&funder_id)
                .expect("No funder balance found");
            cur_funder_balance += amount_to_refund;
            self.user_balances.insert(&funder_id, &cur_funder_balance);
        } else {
            near_sdk::log!(
                "Skipping the refund to funder: {:?} claim success: {:?} refund to deposit?: {:?}",
                funder_id,
                claim_succeeded,
                fc_data.refund_to_deposit.unwrap_or(false)
            );
        }

        self.internal_fc_execute(fc_data, amount_to_refund, account_id, drop_id);
        claim_succeeded
    }

    /// Internal method for deleting the used key and removing / returning linkdrop data.
    /// If drop is none, simulate a panic.
    fn process_claim(
        &mut self,
    ) -> (
        // Drop containing all data
        Option<Drop>,
        // Drop ID for the drop
        Option<DropId>,
        // Balance for the 
        Option<Balance>,
        Option<String>,
        Option<Balance>,
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
        // Ensure only the current contract is calling the method using the access key
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
        let drop_id = self
            .drop_id_for_pk
            .get(&signer_pk)
            .expect("No drop ID found for PK");
        // Remove the drop. If the drop shouldn't be removed, we re-insert later.
        // Panic doesn't affect allowance
        let mut drop = self.drop_for_id.remove(&drop_id).expect("drop not found");
        // Remove the pk from the drop's set and check for key usage.
        // Panic doesn't affect allowance
        let mut key_usage = drop.pks.remove(&signer_pk).unwrap();

        // Ensure there's enough claims left for the key to be used. (this *should* only happen in NFT or FT cases)
        if drop.num_claims_registered < 1 || prepaid_gas != drop.required_gas_attached {
            used_gas = env::used_gas();

            let amount_to_decrement =
                (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
            if drop.num_claims_registered < 1 {
                near_sdk::log!("Not enough claims left for the drop. Decrementing allowance by {}. Used GAS: {}", amount_to_decrement, used_gas.0);
            } else {
                near_sdk::log!("Prepaid GAS different than what is specified in the drop: {}. Decrementing allowance by {}. Used GAS: {}", drop.required_gas_attached.0, amount_to_decrement, used_gas.0);
            }

            key_usage.allowance -= amount_to_decrement;
            near_sdk::log!("Allowance is now {}", key_usage.allowance);
            drop.pks.insert(&signer_pk, &key_usage);
            self.drop_for_id.insert(&drop_id, &drop);
            return (None, None, None, None, None);
        }

        // Ensure enough time has passed if a start timestamp was specified in the config.
        let current_timestamp = env::block_timestamp();
        let desired_timestamp = drop
            .drop_config
            .start_timestamp
            .unwrap_or(current_timestamp);

        if current_timestamp < desired_timestamp {
            used_gas = env::used_gas();

            let amount_to_decrement =
                (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
            near_sdk::log!("Drop isn't claimable until {}. Current timestamp is {}. Decrementing allowance by {}. Used GAS: {}", desired_timestamp, current_timestamp, amount_to_decrement, used_gas.0);

            key_usage.allowance -= amount_to_decrement;
            near_sdk::log!("Allowance is now {}", key_usage.allowance);
            drop.pks.insert(&signer_pk, &key_usage);
            self.drop_for_id.insert(&drop_id, &drop);
            return (None, None, None, None, None);
        }

        // Default the token ID to none and return / remove the next token ID if it's an NFT drop
        let mut token_id = None;
        // Default the storage for longest to be none and return the actual value if it's an NFT drop
        let mut storage_for_longest = None;

        // If it's an NFT drop get the token ID and remove it from the set. Also set the storage for longest
        match &mut drop.drop_type {
            DropType::NFT(data) => {
                token_id = data.token_ids.iter().next();
                data.token_ids.remove(token_id.as_ref().unwrap());
                storage_for_longest = Some(data.storage_for_longest);
            }
            _ => {}
        };

        // Default the should delete variable to true. If there's a case where it shouldn't, change the bool.
        let mut should_delete = true;
        near_sdk::log!(
            "Key usage last used: {:?} Num uses: {:?} (before)",
            key_usage.last_used,
            key_usage.num_uses
        );

        // Ensure the key is within the interval if specified
        if let Some(interval) = drop.drop_config.usage_interval {
            near_sdk::log!(
                "Current timestamp {} last used: {} subs: {} interval: {}",
                current_timestamp,
                key_usage.last_used,
                current_timestamp - key_usage.last_used,
                interval
            );

            if (current_timestamp - key_usage.last_used) < interval
                || key_usage.allowance < prepaid_gas.0 as u128 * self.yocto_per_gas
            {
                used_gas = env::used_gas();

                let amount_to_decrement =
                    (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                if (current_timestamp - key_usage.last_used) < interval {
                    near_sdk::log!("Not enough time has passed since the key was last used. Decrementing allowance by {}. Used GAS: {}", amount_to_decrement, used_gas.0);
                } else {
                    near_sdk::log!("Not enough allowance on the key {}. Decrementing allowance by {} Used GAS: {}", key_usage.allowance, amount_to_decrement, used_gas.0);
                }

                key_usage.allowance -= amount_to_decrement;
                near_sdk::log!("Allowance is now {}", key_usage.allowance);
                drop.pks.insert(&signer_pk, &key_usage);
                self.drop_for_id.insert(&drop_id, &drop);
                return (None, None, None, None, None);
            }

            near_sdk::log!("Enough time has passed for key to be used. Setting last used to current timestamp {}", current_timestamp);
            key_usage.last_used = current_timestamp;
        }

        // No uses left! The key should be deleted
        if key_usage.num_uses == 1 {
            near_sdk::log!("Key has no uses left. It will be deleted");
            self.drop_id_for_pk.remove(&signer_pk);
        } else {
            key_usage.num_uses -= 1;
            key_usage.allowance -= drop.required_gas_attached.0 as u128 * self.yocto_per_gas;
            near_sdk::log!(
                "Key has {} uses left. Decrementing allowance by {}. Allowance left: {}",
                key_usage.num_uses,
                drop.required_gas_attached.0 as u128 * self.yocto_per_gas,
                key_usage.allowance
            );

            drop.pks.insert(&signer_pk, &key_usage);
            should_delete = false;
        }

        drop.num_claims_registered -= 1;

        // If there are keys still left in the drop, add the drop back in with updated data
        if !drop.pks.is_empty() {
            // Add drop back with the updated data.
            self.drop_for_id.insert(&drop_id, &drop);
        } else {
            // Remove the drop ID from the funder's list if the drop is now empty
            self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
        }

        // Calculate the storage being freed. initial - final should be >= 0 since final should be smaller than initial.
        let final_storage = env::storage_usage();
        let total_storage_freed =
            Balance::from(initial_storage - final_storage) * env::storage_byte_cost();

        if should_delete {
            // Amount to refund is the current allowance less the current execution's max GAS
            let amount_to_refund =
                key_usage.allowance - drop.required_gas_attached.0 as u128 * self.yocto_per_gas;
            near_sdk::log!(
                "Key being deleted. Allowance Currently: {}. Will refund: {}",
                key_usage.allowance,
                amount_to_refund
            );
            // Get the funder's balance and increment it by the amount to refund
            let mut cur_funder_balance = self
                .user_balances
                .get(&drop.funder_id)
                .expect("No funder balance found");
            cur_funder_balance += amount_to_refund;
            self.user_balances
                .insert(&drop.funder_id, &cur_funder_balance);

            // Delete the key
            Promise::new(env::current_account_id()).delete_key(signer_pk);
        }

        // Return the drop and optional token ID with how much storage was freed
        (
            Some(drop),
            Some(drop_id),
            Some(total_storage_freed),
            token_id,
            storage_for_longest,
        )
    }
}
