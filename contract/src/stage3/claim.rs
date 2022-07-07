use near_sdk::GasWeight;

use crate::*;

#[near_bindgen]
impl DropZone {
    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();
        require!(prepaid_gas <= ATTACHED_GAS_FROM_WALLET, &format!("cannot attach more than {:?} GAS", ATTACHED_GAS_FROM_WALLET));

        env::log_str(&format!("Beginning of regular claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return drop data and optional token ID for nft drops. Also return the storage freed.
        let (drop_data, storage_freed, token_id, storage_for_longest) = self.process_claim();

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("in regular claim right before transfer: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Should we refund send back the $NEAR since an account isn't being created and just send the assets to the claiming account?
        let account_to_transfer = if drop_data.drop_config.refund_if_claim.unwrap_or(false) == true {drop_data.funder_id.clone()} else {account_id.clone()};

        // TODO: only transfer if balance is > 0
        // Send the account ID the desired balance.
        let promise = Promise::new(account_to_transfer).transfer(drop_data.balance.0);
        
        // Determine what callback we should use depending on the drop type
        match drop_data.drop_type {
            DropType::FC(data) => {
                if let Some(gas) = data.gas_to_attach {
                    // Default amount to refund to be everything except balance (and FC deposit) and burnt GAS
                    let burnt_gas = (gas.0 + GAS_OFFSET_IF_FC_EXECUTE.0) as u128;
                    let total_allowance = BASE_ACCESS_KEY_ALLOWANCE + (drop_data.drop_config.max_claims_per_key - 1) as u128 * burnt_gas;
                    let allowance_refund_per_key = (total_allowance - burnt_gas*drop_data.drop_config.max_claims_per_key as u128) / drop_data.drop_config.max_claims_per_key as u128;
                    
                    let amount_to_refund =  allowance_refund_per_key + ACCESS_KEY_STORAGE + storage_freed;
                    
                    env::log_str(&format!(
                        "Refund Amount: {}, 
                        Allowance refund per key: {},
                        Total allowance: {},
                        Burnt GAS: {}, 
                        Access Key Storage: {}, 
                        Storage Used: {}", 
                        yocto_to_near(amount_to_refund), 
                        yocto_to_near(allowance_refund_per_key),
                        yocto_to_near(total_allowance),
                        yocto_to_near(burnt_gas), 
                        yocto_to_near(ACCESS_KEY_STORAGE), 
                        yocto_to_near(storage_freed))
                    );
                    
                    // Add the refund to the deposit
                    if !data.refund_to_deposit.unwrap_or(false) {
                        // Refunding
                        env::log_str(&format!("Refunding funder: {:?} balance For amount: {:?}", drop_data.funder_id, yocto_to_near(amount_to_refund)));
                        // Get the funder's balance and increment it by the amount to refund
                        let mut cur_funder_balance = self.user_balances.get(&drop_data.funder_id).expect("No funder balance found");
                        cur_funder_balance += amount_to_refund;
                        self.user_balances.insert(&drop_data.funder_id, &cur_funder_balance);
                    } else {
                        env::log_str(&format!("Skipping the refund to funder: {:?} refund to deposit?: {:?}", drop_data.funder_id, data.refund_to_deposit.unwrap_or(false)));
                    }

                    let mut final_args = data.args.clone();

                    // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
                    if let Some(account_field) = &data.claimed_account_field {
                        final_args.insert_str(final_args.len()-1, &format!(",\"{}\":\"{}\"", account_field, account_id));
                        env::log_str(&format!("Adding claimed account ID to specified field: {:?} in args: {:?}", account_field, data.args));
                    }
                
                    env::log_str(&format!("Attaching Total: {:?} Deposit: {:?} Should Refund?: {:?} Amount To Refund: {:?} With args: {:?}", yocto_to_near(data.deposit.0 + if data.refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}), yocto_to_near(data.deposit.0), data.refund_to_deposit.unwrap_or(false), yocto_to_near(amount_to_refund), final_args));

                    // Call function with the min GAS and deposit. all unspent GAS will be added on top
                    Promise::new(data.receiver.clone()).function_call_weight(
                        data.method.clone(), 
                        final_args.as_bytes().to_vec(), 
                        // The claim is successful so attach the amount to refund to the deposit instead of refunding the funder.
                        data.deposit.0 + if data.refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}, 
                        gas,
                        GasWeight(1)
                    );
                }
                promise.then(
                    // Call on_claim_fc with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                    .on_claim_fc(
                        // Account ID that claimed the linkdrop
                        account_id, 
                        // Account ID that funded the linkdrop
                        drop_data.funder_id, 
                        // Balance associated with the linkdrop
                        drop_data.balance, 
                        // How much storage was freed when the key was claimed
                        storage_freed,
                        // Receiver of the function call
                        data.receiver,
                        // Method to call on the contract
                        data.method,
                        // What args to pass in
                        data.args,
                        // What deposit should we attach
                        data.deposit,
                        // Should the refund be sent to the funder or attached to the deposit
                        data.refund_to_deposit,
                        // Should we add the account ID as part of the args and what key should it live in
                        data.claimed_account_field,
                        // How many claims per key
                        drop_data.drop_config.max_claims_per_key as u128
                    )
                );
            },
            DropType::NFT(data) => {
                promise.then(
                    // Call on_claim_nft with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                    .on_claim_nft(
                        // Account ID that claimed the linkdrop
                        account_id, 
                        // Account ID that funded the linkdrop
                        drop_data.funder_id, 
                        // Balance associated with the linkdrop
                        drop_data.balance, 
                        // How much storage was freed when the key was claimed
                        storage_freed,
                        // How much storage was prepaid to cover the longest token ID being inserted.
                        storage_for_longest.expect("no storage for longest token Id found"),
                        // Sender of the NFT
                        data.nft_sender,
                        // Contract where the NFT is stored
                        data.nft_contract,
                        // Token ID for the NFT
                        token_id.expect("no token ID found"),
                        // How many claims per key
                        drop_data.drop_config.max_claims_per_key as u128
                    )
                );
            },
            DropType::FT(data) => {
                promise.then(
                    // Call on_claim_ft with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                    .on_claim_ft(
                        // Account ID that claimed the linkdrop
                        account_id, 
                        // Account ID that funded the linkdrop
                        drop_data.funder_id, 
                        // Balance associated with the linkdrop
                        drop_data.balance, 
                        // How much storage was freed when the key was claimed
                        storage_freed,
                        // Who sent the FTs?
                        data.ft_sender,
                        // Where are the FTs stored
                        data.ft_contract,
                        // How many FTs should we send
                        data.ft_balance,
                        // How much storage does it cost to register the new account
                        data.ft_storage,
                        // How many claims per key
                        drop_data.drop_config.max_claims_per_key as u128
                    )
                );
            },
            DropType::Simple => {
                promise.then(
                    // Call on_claim_simple with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                    .on_claim_simple(
                        // Account ID that funded the linkdrop
                        drop_data.funder_id, 
                        // Balance associated with the linkdrop
                        drop_data.balance, 
                        // How much storage was freed when the key was claimed
                        storage_freed,
                        // How many claims per key
                        drop_data.drop_config.max_claims_per_key as u128
                    )
                );
            }
        };

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of regular claim function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));
    }

    /// Create new account and and claim tokens to it.
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: AccountId,
        new_public_key: PublicKey,
    ) {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of CAAC used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return drop data and optional token ID for nft drops. Also return the storage freed.
        let (drop_data, storage_freed, token_id, storage_for_longest) = self.process_claim();

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("In CAAC after process claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));
        
        // CCC to the linkdrop contract to create the account with the desired balance as the linkdrop amount
        let promise = ext_linkdrop::ext(self.linkdrop_contract.clone())
            // Attach the balance of the linkdrop along with the exact gas for create account. No unspent GAS is attached.
            .with_attached_deposit(drop_data.balance.0)
            .with_static_gas(GAS_FOR_CREATE_ACCOUNT)
            .with_unused_gas_weight(0)
            .create_account(
                new_account_id.clone(),
                new_public_key,  
            );

        // Determine what callback we should use depending on the drop type
        match drop_data.drop_type {
            DropType::FC(data) => {
                promise.then(
                    // Call on_claim_fc with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                    .on_claim_fc(
                        // Account ID that claimed the linkdrop
                        new_account_id, 
                        // Account ID that funded the linkdrop
                        drop_data.funder_id, 
                        // Balance associated with the linkdrop
                        drop_data.balance, 
                        // How much storage was freed when the key was claimed
                        storage_freed,
                        // Receiver of the function call
                        data.receiver,
                        // Method to call on the contract
                        data.method,
                        // What args to pass in
                        data.args,
                        // What deposit should we attach
                        data.deposit,
                        // Should the refund be sent to the funder or attached to the deposit
                        data.refund_to_deposit,
                        // Should we add the account ID as part of the args and what key should it live in
                        data.claimed_account_field,
                        // How many claims per key
                        drop_data.drop_config.max_claims_per_key as u128
                    )
                );
            },
            DropType::NFT(data) => {
                promise.then(
                    // Call on_claim_nft with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                    .on_claim_nft(
                        // Account ID that claimed the linkdrop
                        new_account_id, 
                        // Account ID that funded the linkdrop
                        drop_data.funder_id, 
                        // Balance associated with the linkdrop
                        drop_data.balance, 
                        // How much storage was freed when the key was claimed
                        storage_freed,
                        // How much storage was prepaid to cover the longest token ID being inserted.
                        storage_for_longest.expect("no storage for longest token Id found"),
                        // Sender of the NFT
                        data.nft_sender,
                        // Contract where the NFT is stored
                        data.nft_contract,
                        // Token ID for the NFT
                        token_id.expect("no token ID found"),
                        // How many claims per key
                        drop_data.drop_config.max_claims_per_key as u128
                    )
                );
            },
            DropType::FT(data) => {
                promise.then(
                    // Call on_claim_ft with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                    .on_claim_ft(
                        // Account ID that claimed the linkdrop
                        new_account_id, 
                        // Account ID that funded the linkdrop
                        drop_data.funder_id, 
                        // Balance associated with the linkdrop
                        drop_data.balance, 
                        // How much storage was freed when the key was claimed
                        storage_freed,
                        // Who sent the FTs?
                        data.ft_sender,
                        // Where are the FTs stored
                        data.ft_contract,
                        // How many FTs should we send
                        data.ft_balance,
                        // How much storage does it cost to register the new account
                        data.ft_storage,
                        // How many claims per key
                        drop_data.drop_config.max_claims_per_key as u128
                    )
                );
            },
            DropType::Simple => {
                promise.then(
                    // Call on_claim_simple with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                    .on_claim_simple(
                        // Account ID that funded the linkdrop
                        drop_data.funder_id, 
                        // Balance associated with the linkdrop
                        drop_data.balance, 
                        // How much storage was freed when the key was claimed
                        storage_freed,
                        // How many claims per key
                        drop_data.drop_config.max_claims_per_key as u128
                    )
                );
            }
        };

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of on CAAC function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

    }

    /// Internal method for deleting the used key and removing / returning linkdrop data.
    fn process_claim(&mut self) -> (Drop, Balance, Option<String>, Option<Balance>) {
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();
        // Ensure only the current contract is calling the method using the access key
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );

        // Get the PK of the signer which should be the contract's function call access key
        let signer_pk = env::signer_account_pk();

        // By default, every key should have a drop ID. If we need to remove the key, remove later.
        let drop_id = self.drop_id_for_pk.get(&signer_pk).expect("No drop ID found for PK");
        // Remove the drop. If the drop shouldn't be removed, we re-insert later.
        let mut drop = self.drop_for_id.remove(&drop_id).expect("drop not found");
        
        // Ensure there's enough claims left for the key to be used. (this *should* only happen in NFT or FT cases)
        require!(drop.num_claims_registered >= 1, "Not enough claims left for the drop.");
        drop.num_claims_registered -= 1;

        // Ensure enough time has passed if a start timestamp was specified in the config.
        let current_timestamp = env::block_timestamp();
        let desired_timestamp = drop.drop_config.start_timestamp.unwrap_or(current_timestamp);
        require!(desired_timestamp >= current_timestamp, &format!("Drop isn't claimable until {}", desired_timestamp));
        
        // Remove the pk from the drop's set and check for key usage
        let key_usage = drop.pks.remove(&signer_pk).unwrap();
        
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
            },
            _ => {}
        };

        // Default the should delete variable to true. If there's a case where it shouldn't, change the bool.
        let mut should_delete = true;
        if let Some(mut usage) = key_usage {
            env::log_str(&format!("Key usage found. Last used: {:?} Num uses: {:?} (before)", usage.last_used, usage.num_uses));
            
            // Ensure the key is within the interval if specified
            if let Some(interval) = drop.drop_config.usage_interval {
                require!((current_timestamp - usage.last_used) >= interval, "Not enough time has passed since the key was last used.");
                env::log_str(&format!("Enough time has passed for key to be used. Setting last used to current timestamp {}", current_timestamp));
                usage.last_used = current_timestamp;
            }
            
            // No uses left! The key should be deleted
            if usage.num_uses == 1 {
                env::log_str("Key has no uses left. It will be deleted");
                self.drop_id_for_pk.remove(&signer_pk);
            } else {
                usage.num_uses -= 1;
                env::log_str(&format!("Key has {} uses left", usage.num_uses));
                drop.pks.insert(&signer_pk, &Some(usage));
                should_delete = false;
            }
        }
        
        // If there are keys still left in the drop, add the drop back in with updated data
        if !drop.pks.is_empty() {
            // Add drop back with the updated data.
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
        } else {
            // Remove the drop ID from the funder's list if the drop is now empty
            self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
        }

        // Calculate the storage being freed. initial - final should be >= 0 since final should be smaller than initial.
        let final_storage = env::storage_usage();
        let total_storage_freed = Balance::from(initial_storage - final_storage) * env::storage_byte_cost();

        if should_delete {
            // Delete the key
            Promise::new(env::current_account_id()).delete_key(signer_pk);
        }
        
        // Return the drop and optional token ID with how much storage was freed
        (drop, total_storage_freed, token_id, storage_for_longest)
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
        // How many claims can each key have
        max_claims_per_key: u128
    ) -> bool {        
        // Get the status of the cross contract call
        let claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Simple on claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        let total_allowance = BASE_ACCESS_KEY_ALLOWANCE + (max_claims_per_key - 1) as u128 * ATTACHED_GAS_FROM_WALLET.0 as u128 * GAS_PRICE;
        let allowance_refund_per_key = (total_allowance - ATTACHED_GAS_FROM_WALLET.0 as u128 * max_claims_per_key * GAS_PRICE) / max_claims_per_key;
        
        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  allowance_refund_per_key + ACCESS_KEY_STORAGE + storage_used ;
        
        env::log_str(&format!(
            "Refund Amount: {}, 
            Allowance refund per key: {},
            Total allowance: {},
            Burnt GAS: {}, 
            Access Key Storage: {}, 
            Storage Used: {}", 
            yocto_to_near(amount_to_refund), 
            yocto_to_near(allowance_refund_per_key),
            yocto_to_near(total_allowance),
            yocto_to_near(ATTACHED_GAS_FROM_WALLET.0 as u128 * GAS_PRICE), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(storage_used))
        );

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance as well: {}", balance.0));
            amount_to_refund += balance.0
        }

        env::log_str(&format!("Refunding funder: {:?} For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
        
        // Get the funder's balance and increment it by the amount to refund
        let mut cur_funder_balance = self.user_balances.get(&funder_id).expect("No funder balance found");
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
        // Who sent the FTs?
        ft_sender: AccountId,
        // Where are the FTs stored
        ft_contract: AccountId,
        // How many FTs should we send
        ft_balance: U128,
        // How much storage does it cost to register the new account
        ft_storage: U128,
        // How many claims can each key have
        max_claims_per_key: u128
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of on claim FT used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Get the status of the cross contract call
        let claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        let total_allowance = BASE_ACCESS_KEY_ALLOWANCE + (max_claims_per_key - 1) as u128 * ATTACHED_GAS_FROM_WALLET.0 as u128 * GAS_PRICE;
        let allowance_refund_per_key = (total_allowance - ATTACHED_GAS_FROM_WALLET.0 as u128 * max_claims_per_key * GAS_PRICE) / max_claims_per_key;
        
        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  allowance_refund_per_key + ACCESS_KEY_STORAGE + storage_used ;
        
        env::log_str(&format!(
            "Refund Amount: {}, 
            Allowance refund per key: {},
            Total allowance: {},
            Burnt GAS: {}, 
            Access Key Storage: {}, 
            Storage Used: {}", 
            yocto_to_near(amount_to_refund), 
            yocto_to_near(allowance_refund_per_key),
            yocto_to_near(total_allowance),
            yocto_to_near(ATTACHED_GAS_FROM_WALLET.0 as u128 * GAS_PRICE), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(storage_used))
        );
        
        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance as well: {}", balance.0));
            amount_to_refund += balance.0
        }

        env::log_str(&format!("Refunding funder: {:?} balance For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
        // Get the funder's balance and increment it by the amount to refund
        let mut cur_funder_balance = self.user_balances.get(&funder_id).expect("No funder balance found");
        cur_funder_balance += amount_to_refund;
        self.user_balances.insert(&funder_id, &cur_funder_balance);

        /*
            Fungible Tokens. 
            - Only send the FTs if the sender ended up sending the contract the tokens.
        */
        // Only send the fungible tokens to the new account if the claim was successful. We return the FTs if it wasn't successful in the else case.
        if claim_succeeded {
            // Create a new batch promise to pay storage and transfer FTs to the new account ID
            let batch_ft_promise_id = env::promise_batch_create(&ft_contract);

            // Pay the required storage as outlined in the AccountData. This will run first and then we send the fungible tokens
            // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "storage_deposit",
                json!({ "account_id": account_id }).to_string().as_bytes(),
                ft_storage.0,
                MIN_GAS_FOR_STORAGE_DEPOSIT,
                GasWeight(1)
            );  

            // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "ft_transfer",
                json!({ "receiver_id": account_id, "amount": ft_balance, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                1,
                MIN_GAS_FOR_FT_TRANSFER,
                GasWeight(1)
            );

            // Create the second batch promise to execute after the batch_ft_promise_id batch is finished executing.
            // It will execute on the current account ID (this contract)
            let batch_ft_resolve_promise_id = env::promise_batch_then(batch_ft_promise_id, &env::current_account_id());

            // Execute a function call as part of the resolved promise index created in promise_batch_then
            // Callback after both the storage was deposited and the fungible tokens were sent
            // Call the function with the min GAS and then attach 3/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_resolve_promise_id,
                "ft_resolve_batch",
                json!({ "amount": ft_balance, "token_sender": ft_sender, "token_contract": ft_contract }).to_string().as_bytes(),
                NO_DEPOSIT,
                MIN_GAS_FOR_RESOLVE_BATCH,
                GasWeight(3)
            );

        } else {
            // Create a new batch promise to pay storage and refund the FTs to the original sender 
            let batch_ft_promise_id = env::promise_batch_create(&ft_contract);

            // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "storage_deposit",
                json!({ "account_id": ft_sender }).to_string().as_bytes(),
                ft_storage.0,
                MIN_GAS_FOR_STORAGE_DEPOSIT,
                GasWeight(1)
            );

            // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "ft_transfer",
                json!({ "receiver_id": ft_sender, "amount": ft_balance, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                1,
                MIN_GAS_FOR_FT_TRANSFER,
                GasWeight(1)
            );

            // Return the result of the batch as the return of the function
            env::promise_return(batch_ft_promise_id);
        }

        claim_succeeded
    }

    #[private]
    /// self callback for a linkdrop loaded with an NFT
    pub fn on_claim_nft(&mut self, 
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
        // How many claims can each key have
        max_claims_per_key: u128
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of on claim NFT used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Get the status of the cross contract call
        let claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        // In addition, we refund them for the cost of storing the longest token ID now that a key has been claimed
        let total_allowance = BASE_ACCESS_KEY_ALLOWANCE + (max_claims_per_key - 1) as u128 * ATTACHED_GAS_FROM_WALLET.0 as u128 * GAS_PRICE;
        let allowance_refund_per_key = (total_allowance - ATTACHED_GAS_FROM_WALLET.0 as u128 * max_claims_per_key * GAS_PRICE) / max_claims_per_key;
        
        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  allowance_refund_per_key + ACCESS_KEY_STORAGE + storage_used + storage_for_longest * env::storage_byte_cost();
        
        env::log_str(&format!(
            "Refund Amount: {}, 
            Allowance refund per key: {},
            Total allowance: {},
            Burnt GAS: {}, 
            Access Key Storage: {}, 
            Storage Used: {}
            Storage for longest: {}", 
            yocto_to_near(amount_to_refund), 
            yocto_to_near(allowance_refund_per_key),
            yocto_to_near(total_allowance),
            yocto_to_near(ATTACHED_GAS_FROM_WALLET.0 as u128 * GAS_PRICE), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(storage_used),
            yocto_to_near(storage_for_longest * env::storage_byte_cost()))
        );
    
        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance as well: {}", balance.0));
            amount_to_refund += balance.0
        }

        env::log_str(&format!("Refunding funder: {:?} balance For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
        // Get the funder's balance and increment it by the amount to refund
        let mut cur_funder_balance = self.user_balances.get(&funder_id).expect("No funder balance found");
        cur_funder_balance += amount_to_refund;
        self.user_balances.insert(&funder_id, &cur_funder_balance);

        /*
            Non Fungible Tokens
        */
        // Only send the NFT to the new account if the claim was successful. We return the NFT if it wasn't successful in the else case.
        if claim_succeeded {
            // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
            ext_nft_contract::ext(nft_contract.clone())
                // Call nft transfer with the min GAS and 1 yoctoNEAR. 1/2 unspent GAS will be added on top
                .with_static_gas(MIN_GAS_FOR_SIMPLE_NFT_TRANSFER)
                .with_attached_deposit(1)
                .nft_transfer(
                    account_id.clone(), 
                    token_id.clone(),
                    None,
                    Some("Linkdropped NFT".to_string()),
                )
            // We then resolve the promise and call nft_resolve_transfer on our own contract
            .then(
                // Call resolve transfer with the min GAS and no deposit. 1/2 unspent GAS will be added on top
                Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_RESOLVE_TRANSFER)
                    .nft_resolve_transfer(
                        token_id,
                        nft_sender,
                        nft_contract,
                    )
            );
        } else {
            // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
            ext_nft_contract::ext(nft_contract)
                // Call nft transfer with the min GAS and 1 yoctoNEAR. all unspent GAS will be added on top
                .with_static_gas(MIN_GAS_FOR_SIMPLE_NFT_TRANSFER)
                .with_attached_deposit(1)
                .nft_transfer(
                    nft_sender, 
                    token_id,
                    None,
                    Some("Linkdropped NFT".to_string()),
                );
        }

        claim_succeeded
    }

    #[private]
    /// self callback checks if account was created successfully or not. If yes, refunds excess storage, sends NFTs, FTs etc..
    pub fn on_claim_fc(&mut self, 
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        funder_id: AccountId, 
        // Balance associated with the linkdrop
        balance: U128, 
        // How much storage was freed when the key was claimed
        storage_used: Balance,
        // Receiver of the function call
        receiver: AccountId,
        // Method to call on the contract
        method: String,
        // What args to pass in
        args: String,
        // What deposit should we attach
        deposit: U128,
        // Should the refund be sent to the funder or attached to the deposit
        add_refund_to_deposit: Option<bool>,
        // Should we add the account ID as part of the args and what key should it live in
        claimed_account_field: Option<String>,
        // How many claims can each key have
        max_claims_per_key: u128
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of on claim Function Call used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Get the status of the cross contract call
        let claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        let total_allowance = BASE_ACCESS_KEY_ALLOWANCE + (max_claims_per_key - 1) as u128 * ATTACHED_GAS_FROM_WALLET.0 as u128 * GAS_PRICE;
        let allowance_refund_per_key = (total_allowance - ATTACHED_GAS_FROM_WALLET.0 as u128 * max_claims_per_key * GAS_PRICE) / max_claims_per_key;
        
        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  allowance_refund_per_key + ACCESS_KEY_STORAGE + storage_used ;
        
        env::log_str(&format!(
            "Refund Amount: {}, 
            Allowance refund per key: {},
            Total allowance: {},
            Burnt GAS: {}, 
            Access Key Storage: {}, 
            Storage Used: {}", 
            yocto_to_near(amount_to_refund), 
            yocto_to_near(allowance_refund_per_key),
            yocto_to_near(total_allowance),
            yocto_to_near(ATTACHED_GAS_FROM_WALLET.0 as u128 * GAS_PRICE), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(storage_used))
        );
        
        // If not successful, the balance and deposit is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance: {} and deposit: {}", balance.0, deposit.0));
            amount_to_refund += balance.0 + deposit.0
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
        if !claim_succeeded || (!add_refund_to_deposit.unwrap_or(false) && claim_succeeded) {
            // Refunding
            env::log_str(&format!("Refunding funder: {:?} balance For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
            // Get the funder's balance and increment it by the amount to refund
            let mut cur_funder_balance = self.user_balances.get(&funder_id).expect("No funder balance found");
            cur_funder_balance += amount_to_refund;
            self.user_balances.insert(&funder_id, &cur_funder_balance);
        } else {
            env::log_str(&format!("Skipping the refund to funder: {:?} claim success: {:?} refund to deposit?: {:?}", funder_id, claim_succeeded, add_refund_to_deposit.unwrap_or(false)));
        }

        /*
            Function Calls
        */
        // Only call the function if the claim was successful.
        if claim_succeeded {
            let mut final_args = args.clone();

            // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
            if let Some(account_field) = claimed_account_field {
                final_args.insert_str(final_args.len()-1, &format!(",\"{}\":\"{}\"", account_field, account_id));
                env::log_str(&format!("Adding claimed account ID to specified field: {:?} in args: {:?}", account_field, args));
            }
        
            env::log_str(&format!("Attaching Total: {:?} Deposit: {:?} Should Refund?: {:?} Amount To Refund: {:?} With args: {:?}", yocto_to_near(deposit.0 + if add_refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}), yocto_to_near(deposit.0), add_refund_to_deposit.unwrap_or(false), yocto_to_near(amount_to_refund), final_args));

            // Call function with the min GAS and deposit. all unspent GAS will be added on top
            Promise::new(receiver).function_call_weight(
                method, 
                final_args.as_bytes().to_vec(), 
                // The claim is successful so attach the amount to refund to the deposit instead of refunding the funder.
                deposit.0 + if add_refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}, 
                Gas(0),
                GasWeight(1)
            );
        }

        claim_succeeded
    }
}