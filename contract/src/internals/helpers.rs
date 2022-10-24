use crate::*;

const GAS_PER_CCC: Gas = Gas(5_000_000_000_000); // 5 TGas
const RECEIPT_GAS_COST: Gas = Gas(2_500_000_000_000); // 2.5 TGas

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_account_id(account_id: &String) -> CryptoHash {
    env::sha256_array(account_id.as_bytes())
}

/// Helper function to convert yoctoNEAR to $NEAR with 7 decimals of precision.
pub(crate) fn yocto_to_near(yocto: u128) -> f64 {
    //10^17 yoctoNEAR (1 NEAR would be 10_000_000). This is to give a precision of 7 decimal places.
    let formatted_near = yocto / 100_000_000_000_000_000;
    let near = formatted_near as f64 / 10_000_000f64;

    near
}

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn check_promise_result() -> bool {
    if let PromiseResult::Successful(value) = env::promise_result(0) {
        // If the value was empty string, then it was a regular claim
        if value.is_empty() {
            near_sdk::log!("received empty string as success value");
            true
        } else {
            if let Ok(account_created) = near_sdk::serde_json::from_slice::<bool>(&value) {
                //if we need don't need to return the token, we simply return true meaning everything went fine
                near_sdk::log!("received value of {} as success value", account_created);
                account_created
            } else {
                near_sdk::log!("did not receive boolean from success value");
                false
            }
        }
    } else {
        near_sdk::log!("promise result not successful");
        false
    }
}

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn assert_valid_drop_config(drop_config: &Option<DropConfig>) {
    if let Some(config) = drop_config {
        near_sdk::log!("Current Block Timestamp: {}", env::block_timestamp());
        
        // Assert that if uses per key is passed in, it cannot equal 0
        assert!(
            config.uses_per_key.unwrap_or(1) != 0,
            "Cannot have 0 uses per key for a drop config"
        );

        // Assert that if the claim_interval is some, the start_timestamp is also some
        assert!(
            (config.claim_interval.is_some() && config.start_timestamp.is_none()) == false,
            "If you want to set a claim interval, you must also set a start timestamp"
        );

        // Assert that both the start_timestamp and end timestamps are greater than the current block
        assert!(
            config.start_timestamp.unwrap_or(env::block_timestamp()) >= env::block_timestamp(),
            "The start timestamp must be greater than the current block timestamp"
        );
        assert!(
            config.end_timestamp.unwrap_or(env::block_timestamp()) >= env::block_timestamp(),
            "The end timestamp must be greater than the current block timestamp"
        );


        // If both the start timestamp and end timestamp are set, ensure that the start timestamp is less than the end timestamp
        if config.start_timestamp.is_some() && config.end_timestamp.is_some() {
            assert!(
                config.start_timestamp.unwrap() < config.end_timestamp.unwrap(),
                "The start timestamp must be less than the end timestamp"
            );
        }
    }
}

impl Keypom {
    /// Internal function to assert that the predecessor is the contract owner
    pub(crate) fn assert_owner(&mut self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
    }

    /// Internal function to assert that the predecessor is the contract owner
    pub(crate) fn assert_claim_timestamps(&mut self, drop_id: DropId, drop: &mut Drop, key_info: &mut KeyInfo, signer_pk: &PublicKey) -> bool {
        // Ensure enough time has passed if a start timestamp was specified in the config.
        let current_timestamp = env::block_timestamp();
        let desired_start_timestamp = drop
            .config
            .clone()
            .and_then(|c| c.start_timestamp)
            .unwrap_or(0);

        if current_timestamp < desired_start_timestamp {
            let used_gas = env::used_gas();

            let amount_to_decrement =
                (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
            near_sdk::log!("Drop isn't claimable until {}. Current timestamp is {}. Decrementing allowance by {}. Used GAS: {}", desired_start_timestamp, current_timestamp, amount_to_decrement, used_gas.0);

            key_info.allowance -= amount_to_decrement;
            near_sdk::log!("Allowance is now {}", key_info.allowance);
            drop.pks.insert(&signer_pk, &key_info);
            self.drop_for_id.insert(&drop_id, &drop);
            return false;
        }

        // Ensure the end timestamp hasn't passed and the key is still usable
        let desired_end_timestamp = drop
            .config
            .clone()
            .and_then(|c| c.end_timestamp)
            .unwrap_or(u64::MAX);
        if current_timestamp > desired_end_timestamp {
            let used_gas = env::used_gas();

            let amount_to_decrement =
                (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
            near_sdk::log!("Drop claim period passed. Claimable up until {}. Current timestamp is {}. Decrementing allowance by {}. Used GAS: {}", desired_end_timestamp, current_timestamp, amount_to_decrement, used_gas.0);

            key_info.allowance -= amount_to_decrement;
            near_sdk::log!("Allowance is now {}", key_info.allowance);
            drop.pks.insert(&signer_pk, &key_info);
            self.drop_for_id.insert(&drop_id, &drop);
            return false;
        }

        // Ensure the key is within the interval if specified
        if let Some(interval) = drop.config.clone().and_then(|c| c.throttle_timestamp) {
            near_sdk::log!(
                "Current timestamp {} last used: {} subs: {} interval: {}",
                current_timestamp,
                key_info.last_used,
                current_timestamp - key_info.last_used,
                interval
            );

            if (current_timestamp - key_info.last_used) < interval {
                let used_gas = env::used_gas();

                let amount_to_decrement =
                    (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                near_sdk::log!("Not enough time has passed since the key was last used. Decrementing allowance by {}. Used GAS: {}", amount_to_decrement, used_gas.0);

                key_info.allowance -= amount_to_decrement;
                near_sdk::log!("Allowance is now {}", key_info.allowance);
                drop.pks.insert(&signer_pk, &key_info);
                self.drop_for_id.insert(&drop_id, &drop);
                return false;
            }

            near_sdk::log!("Enough time has passed for key to be used. Setting last used to current timestamp {}", current_timestamp);
            key_info.last_used = current_timestamp;
        }

        // Ensure the key is within the claim interval if specified
        if let Some(interval) = drop.config.clone().and_then(|c| c.claim_interval) {
            let start_timestamp = drop.config.clone().and_then(|c| c.start_timestamp).unwrap();
            let total_num_claims = (env::block_timestamp() - start_timestamp) / interval;
            let uses_per_key = drop.config.clone().and_then(|c| c.uses_per_key).unwrap_or(0);
            let claims_left = total_num_claims + key_info.remaining_uses - uses_per_key ;
           
            near_sdk::log!(
                "Current timestamp {} start timestamp: {} claim interval: {} total num claims: {} total uses per key: {} remaining uses: {} num remaining claims: {}",
                current_timestamp,
                start_timestamp,
                interval,
                total_num_claims,
                uses_per_key,
                key_info.remaining_uses,
                claims_left
            );

            if claims_left < 1 {
                let used_gas = env::used_gas();

                let amount_to_decrement =
                    (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                near_sdk::log!("Not enough time has passed before the key can be used. Decrementing allowance by {}. Used GAS: {}", amount_to_decrement, used_gas.0);

                key_info.allowance -= amount_to_decrement;
                near_sdk::log!("Allowance is now {}", key_info.allowance);
                drop.pks.insert(&signer_pk, &key_info);
                self.drop_for_id.insert(&drop_id, &drop);
                return false;
            }
        }

        true
    }

    /// Internal function to register Keypom on a given FT contract
    pub(crate) fn internal_register_ft_contract(&mut self, ft_contract_id: &AccountId, storage_required: u128, account_to_refund: &AccountId, refund_balance: bool) {
        // Check if the ft contract is already in the registered ft contracts list
        if !self.registered_ft_contracts.contains(ft_contract_id) {
            near_sdk::log!("FT contract not registered. Performing cross contract call to {} and inserting back into set", ft_contract_id);

            // Perform a cross contract call to fire and forget. Attach the storage required
            ext_ft_contract::ext(ft_contract_id.clone())
                // Call storage balance bounds with exactly this amount of GAS. No unspent GAS will be added on top.
                .with_static_gas(MIN_GAS_FOR_FT_TRANSFER)
                .with_attached_deposit(storage_required)
                .storage_deposit(Some(env::current_account_id()), None);
            
            self.registered_ft_contracts.insert(ft_contract_id);
            return;
        }

        // If we should refund the account's balance, do it here. Otherwise, just transfer the funds directly.
        if refund_balance {
            let mut cur_user_bal = self.user_balances.get(account_to_refund).unwrap_or(0);
            cur_user_bal += storage_required;
            near_sdk::log!("FT contract already registered. Refunding user balance for {}. Balance is now {}", yocto_to_near(storage_required), yocto_to_near(cur_user_bal));
            self.user_balances.insert(account_to_refund, &cur_user_bal);
            return;
        }

        near_sdk::log!("FT contract already registered. Transferring user for: {}", yocto_to_near(storage_required));
        Promise::new(account_to_refund.clone()).transfer(storage_required);
    }

    /// Internal function to force remove a drop from the contract's state
    pub(crate) fn internal_remove_drop(&mut self, drop_id: &u128, public_keys: Vec<PublicKey>) -> AccountId {
        // Remove the drop
        let mut drop = self.drop_for_id.remove(drop_id).expect("drop not found");
        // Clear the map
        drop.pks.clear();
        let owner_id = drop.owner_id.clone();

        // Remove the drop ID from the funder's list
        self.internal_remove_drop_for_funder(&drop.owner_id, &drop_id);

        // Loop through the keys and remove the public keys' mapping
        for pk in public_keys {
            self.drop_id_for_pk.remove(&pk.clone());
        };

        // Return the owner ID
        owner_id
    }

    /// Used to calculate the base allowance needed given attached GAS
    pub(crate) fn calculate_base_allowance(&self, attached_gas: Gas) -> u128 {
        // Get the number of CCCs you can make with the attached GAS
        let calls_with_gas = (attached_gas.0 / GAS_PER_CCC.0) as f32;
        // Get the constant used to pessimistically calculate the required allowance
        let pow_outcome = 1.03_f32.powf(calls_with_gas);

        // Get the required GAS based on the calculated constant
        let required_allowance = ((attached_gas.0 + RECEIPT_GAS_COST.0) as f32 * pow_outcome
            + RECEIPT_GAS_COST.0 as f32) as u128
            * self.yocto_per_gas;
        near_sdk::log!(
            "{} calls with {} attached GAS. Pow outcome: {}. Required Allowance: {}",
            calls_with_gas,
            attached_gas.0,
            pow_outcome,
            required_allowance
        );

        required_allowance
    }

    /// Add a drop ID to the set of drops a funder has
    pub(crate) fn internal_add_drop_to_funder(&mut self, account_id: &AccountId, drop_id: &DropId) {
        //get the set of drops for the given account
        let mut drop_set = self.drop_ids_for_owner.get(account_id).unwrap_or_else(|| {
            //if the account doesn't have any drops, we create a new unordered set
            UnorderedSet::new(StorageKey::DropIdsForFunderInner {
                //we get a new unique prefix for the collection
                account_id_hash: hash_account_id(&account_id.to_string()),
            })
        });

        //we insert the drop ID into the set
        drop_set.insert(drop_id);

        //we insert that set for the given account ID.
        self.drop_ids_for_owner.insert(account_id, &drop_set);
    }

    //remove a drop ID for a funder (internal method_name and can't be called directly via CLI).
    pub(crate) fn internal_remove_drop_for_funder(
        &mut self,
        account_id: &AccountId,
        drop_id: &DropId,
    ) {
        //we get the set of drop IDs that the funder has
        let mut drop_set = self
            .drop_ids_for_owner
            .get(account_id)
            //if there is no set of drops for the owner, we panic with the following message:
            .expect("No Drops found for the funder");

        //we remove the the drop ID from  the set of drops
        drop_set.remove(drop_id);

        //if the set is now empty, we remove the funder from the drop_ids_for_owner collection
        if drop_set.is_empty() {
            self.drop_ids_for_owner.remove(account_id);
        } else {
            //if the key set is not empty, we simply insert it back for the funder ID.
            self.drop_ids_for_owner.insert(account_id, &drop_set);
        }
    }

    /// Internal function for executing the callback code either straight up or using `.then` for a passed in promise
    pub(crate) fn internal_execute(
        &mut self,
        drop_data: Drop,
        drop_id: DropId,
        cur_key_info: KeyInfo,
        account_id: AccountId,
        storage_freed: u128,
        token_id: Option<String>,
        auto_withdraw: bool,
        promise: Option<Promise>,
    ) {
        macro_rules! resolve_promise_or_call {
            ( $func:ident ( $($call:tt)* ) ) => {
                if let Some(promise) = promise {
                    promise.then(
                        // Call on_claim_fc with all unspent GAS + min gas for on claim. No attached attached_deposit.
                        Self::ext(env::current_account_id())
                        .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                        .$func(
                            $($call)*
                            // Executing the function and treating it like a callback.
                            false,
                        )
                    );
                } else {
                    // We're not dealing with a promise so we simply execute the function.
                    self.$func(
                        $($call)*
                        // Executing the function and treating it NOT like a callback.
                        true,
                    );
                }
            }
        }
        // Determine what callback we should use depending on the drop type
        match drop_data.drop_type {
            DropType::FunctionCall(data) => {
                // If we're dealing with a promise, execute the callback
                resolve_promise_or_call!(on_claim_fc(
                    // Account ID that claimed the linkdrop
                    account_id,
                    // Account ID that funded the linkdrop
                    drop_data.owner_id,
                    // Balance associated with the linkdrop
                    U128(drop_data.deposit_per_use),
                    // How much storage was freed when the key was claimed
                    storage_freed,
                    // FC Data
                    data,
                    // Drop ID
                    drop_id,
                    // Current number of claims left on the key before decrementing
                    cur_key_info,
                    // Maximum number of claims
                    drop_data.config.and_then(|c| c.uses_per_key).unwrap_or(1),
                    // Is it an auto withdraw case
                    auto_withdraw,
                ));
            }
            DropType::NonFungibleToken(data) => {
                resolve_promise_or_call!(on_claim_nft(
                    // Account ID that claimed the linkdrop
                    account_id,
                    // Account ID that funded the linkdrop
                    drop_data.owner_id,
                    // Balance associated with the linkdrop
                    U128(drop_data.deposit_per_use),
                    // How much storage was freed when the key was claimed
                    storage_freed,
                    // Sender of the NFT
                    data.sender_id,
                    // Contract where the NFT is stored
                    data.contract_id,
                    // Token ID for the NFT
                    token_id.expect("no token ID found"),
                    // Is it an auto withdraw case
                    auto_withdraw,
                ));
            }
            DropType::FungibleToken(data) => {
                resolve_promise_or_call!(on_claim_ft(
                    // Account ID that claimed the linkdrop
                    account_id,
                    // Account ID that funded the linkdrop
                    drop_data.owner_id,
                    // Balance associated with the linkdrop
                    U128(drop_data.deposit_per_use),
                    // How much storage was freed when the key was claimed
                    storage_freed,
                    // FT Data to be used
                    data,
                    // Is it an auto withdraw case
                    auto_withdraw,
                ));
            }
            DropType::Simple => {
                promise.unwrap().then(
                    // Call on_claim_simple with all unspent GAS + min gas for on claim. No attached attached_deposit.
                    Self::ext(env::current_account_id())
                        .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                        .on_claim_simple(
                            // Account ID that funded the linkdrop
                            drop_data.owner_id,
                            // Balance associated with the linkdrop
                            U128(drop_data.deposit_per_use),
                            // How much storage was freed when the key was claimed
                            storage_freed,
                            // Is it an auto withdraw case
                            auto_withdraw,
                        ),
                );
            }
        };
    }
}
