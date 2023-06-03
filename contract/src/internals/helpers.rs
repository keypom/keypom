use std::str::FromStr;

use near_sdk::env::sha256;
use std::convert::{TryFrom};

use serde_json::{Value};

use crate::{*, stage1::KeypomArgs};

const GAS_PER_CCC: Gas = Gas(5_000_000_000_000); // 5 TGas
const RECEIPT_GAS_COST: Gas = Gas(2_500_000_000_000); // 2.5 TGas

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_account_id(account_id: &String) -> CryptoHash {
    env::sha256_array(account_id.as_bytes())
}

pub(crate) fn rem_first_and_last(value: &mut String) {
    value.pop();
    value.remove(0);
}

/// Take a token ID and return the drop ID and key nonce based on the `:` delimiter.
pub(crate) fn parse_token_id(token_id: &TokenId) -> (DropId, u64) {
    let delimiter = ":";
    let split: Vec<&str> = token_id.split(delimiter).collect();
    let drop_id = u128::from_str(split[0]).unwrap();
    let key_nonce = u64::from_str(split[1]).unwrap();
    return (drop_id, key_nonce);
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
pub(crate) fn assert_valid_drop_config(drop_config: &Option<JsonDropConfig>, drop_id: &DropId, funder: &AccountId) -> Option<DropConfig> {
    let mut actual_config = None;

    if let Some(config) = drop_config.clone() {
        actual_config = Some(DropConfig {
            uses_per_key: config.uses_per_key,
            nft_metadata: config.nft_metadata,
            nft_royalty: config.nft_royalty,
            time: None,
            usage: config.usage,
            sale: None,
            root_account_id: config.root_account_id
        });

        if let Some(sale) = &config.sale {
            let mut actual_allowlist = None;
            let mut actual_blocklist = None;
            
            // Loop through and add all the accounts to the allow list
            if let Some(list) = &sale.allowlist {
                let mut allowlist = UnorderedSet::new(StorageKey::PubSaleAllowlist {
                    //we get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&format!("allowlist-{}{}", drop_id, funder)),
                });
                for account in list {
                    allowlist.insert(account);
                }

                actual_allowlist = Some(allowlist);
            }

            // Loop through and add all the accounts to the allow list
            if let Some(list) = &sale.blocklist {
                let mut blocklist = UnorderedSet::new(StorageKey::PubSaleBlocklist {
                    //we get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&format!("blocklist-{}{}", drop_id, funder)),
                });
                for account in list {
                    blocklist.insert(account);
                }

                actual_blocklist = Some(blocklist);
            }

            let sale = PublicSaleConfig {
                max_num_keys: sale.max_num_keys,
                price_per_key: sale.price_per_key.map(|p| p.0),
                allowlist: actual_allowlist,
                blocklist: actual_blocklist,
                auto_withdraw_funds: sale.auto_withdraw_funds,
                start: sale.start,
                end: sale.end,
            };

            actual_config.as_mut().unwrap().sale = Some(sale);
        }

        near_sdk::log!("Current Block Timestamp: {}", env::block_timestamp());

        // Assert that if uses per key is passed in, it cannot equal 0
        assert!(
            config.uses_per_key.unwrap_or(1) != 0,
            "Cannot have 0 uses per key for a drop config"
        );

        if let Some(time_data) = config.time {
            // Assert that if the claim_interval is some, the start_timestamp is also some
            assert!(
                (time_data.interval.is_some() && time_data.start.is_none()) == false,
                "If you want to set a claim interval, you must also set a start timestamp"
            );

            // Assert that both the start_timestamp and end timestamps are greater than the current block
            assert!(
                time_data.start.unwrap_or(env::block_timestamp()) >= env::block_timestamp(),
                "The start timestamp must be greater than the current block timestamp"
            );
            assert!(
                time_data.end.unwrap_or(env::block_timestamp()) >= env::block_timestamp(),
                "The end timestamp must be greater than the current block timestamp"
            );

            // If both the start timestamp and end timestamp are set, ensure that the start timestamp is less than the end timestamp
            if time_data.start.is_some() && time_data.end.is_some() {
                assert!(
                    time_data.start.unwrap() < time_data.end.unwrap(),
                    "The start timestamp must be less than the end timestamp"
                );
            }

            actual_config.as_mut().unwrap().time = Some(time_data);
        }
    }

    actual_config
}

/// Check if the timestamps and allowlists are fulfilled. Return the price per key if it is
pub(crate) fn assert_sale_requirements(sale: &PublicSaleConfig, cur_num_keys: u64, num_keys_to_add: u64) -> u128 {
    // Assert that the current time is between the start and end time
    let cur_time = env::block_timestamp();
    let desired_start = sale.start.unwrap_or(0);
    let desired_end = sale.end.unwrap_or(u64::MAX);
    assert!(
        cur_time >= desired_start && cur_time <= desired_end,
        "Public Sale Has Ended"
    );

    // Assert that the current number of keys is less than the max number of keys
    let max_num_keys = sale.max_num_keys.unwrap_or(u64::MAX);
    assert!(
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

    // Return the price per key
    return sale.price_per_key.unwrap_or(0) * num_keys_to_add as u128;
}

/// Helper function to convert yoctoNEAR to $NEAR with 7 decimals of precision.
pub(crate) fn insert_keypom_args_to_ca_payload(mut payload: String, keypom_args: KeypomArgs, new_account_id: String, drop_id: String, key_id: String, funder_id: String) -> String {
    payload.insert_str(
        payload.len() - 1,
        &format!(
            ",\"keypom_args\":{}",
            near_sdk::serde_json::to_string(&keypom_args).unwrap()
        ),
    );

    near_sdk::log!("payload with keypom args{}", payload);

    // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
    if let Some(field) = keypom_args.account_id_field.as_ref() {
        payload.insert_str(
            payload.len() - 1,
            &format!(",\"{}\":\"{}\"", field, new_account_id),
        );
    }

    // Add drop_id
    if let Some(field) = keypom_args.drop_id_field.as_ref() {
        payload.insert_str(
            payload.len() - 1,
            &format!(",\"{}\":\"{}\"", field, drop_id),
        );
    }

    // Add the key_id
    if let Some(field) = keypom_args.key_id_field.as_ref() {
        payload.insert_str(
            payload.len() - 1,
            &format!(",\"{}\":\"{}\"", field, key_id),
        );
    }

    // Add the funder_id
    if let Some(field) = keypom_args.funder_id_field.as_ref() {
        payload.insert_str(
            payload.len() - 1,
            &format!(",\"{}\":\"{}\"", field, funder_id),
        );
    }

    near_sdk::log!("payload after all insertions{}", payload);

    payload
}

/// b will overwrite a and `a` will be mutated
pub(crate) fn merge_json(a: &mut Value, b: &Value) {
    match (a, b) {
        (&mut Value::Object(ref mut a), &Value::Object(ref b)) => {
            for (k, v) in b {
                merge_json(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

impl Keypom {
    /// Helper function to add any attached deposit to the user's balance
    pub(crate) fn attached_deposit_to_user_balance(&mut self, owner_id: &AccountId) -> (u128, u128) {
        // Get the current balance of the funder.
        let mut current_user_balance = self.user_balances.get(owner_id).unwrap_or(0);
        let near_attached = env::attached_deposit();
        // Add the attached deposit to their balance
        current_user_balance += near_attached;
        self.user_balances.insert(owner_id, &current_user_balance);
        (current_user_balance, near_attached)
    }

    /// Internal function to modify the user's balance. Defaults to adding the amount but decrement can also be specified
    pub(crate) fn internal_modify_user_balance(&mut self, account_id: &AccountId, amount: u128, decrement: bool) {
        // Get the balance of the account (if the account isn't in the map we default to a balance of 0)
        let mut balance: u128 = self
            .user_balances
            .get(account_id)
            .unwrap_or(0);

        // Either add or subtract the amount from the balance depending on whether or not decrement was passed in
        if decrement == true {
            near_sdk::log!(
                "User balance decremented by {}. Old: {} new: {}",
                yocto_to_near(amount),
                yocto_to_near(balance),
                yocto_to_near(balance - amount)
            );    
            balance -= amount;
        } else {
            near_sdk::log!(
                "User balance incremented by {}. Old: {} new: {}",
                yocto_to_near(amount),
                yocto_to_near(balance),
                yocto_to_near(balance + amount)
            );  
            balance += amount;
        }

        // Insert the balance back into the map for that account ID
        self.user_balances
            .insert(account_id, &balance);
    }

    /// Internal function to assert that the predecessor is the contract owner
    pub(crate) fn assert_owner(&mut self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
    }

    /// Internal function to assert that the predecessor is the contract owner
    pub(crate) fn assert_key_password(
        &mut self,
        pw: Option<String>,
        drop_id: DropId,
        drop: &mut Drop,
        key_info: &mut KeyInfo,
        cur_use: &u64,
        token_id: &String,
    ) -> bool {
        let hashed = sha256(&pw.and_then(|f| hex::decode(f).ok()).unwrap_or(vec![]));

        // If there is a global password per key, check that first
        if let Some(pw) = &key_info.pw_per_key {
            near_sdk::log!("hashed password: {:?}", hashed);
            near_sdk::log!("global password: {:?}", pw);
            if pw != &hashed {
                let used_gas = env::used_gas();

                let amount_to_decrement =
                    (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                near_sdk::log!(
                    "Incorrect password. Decrementing allowance by {}. Used GAS: {}",
                    amount_to_decrement,
                    used_gas.0
                );

                key_info.allowance -= amount_to_decrement;
                near_sdk::log!("Allowance is now {}", key_info.allowance);
                drop.key_info_by_token_id.insert(&token_id, &key_info);
                self.drop_for_id.insert(&drop_id, &drop);
                return false;
            }
        }

        near_sdk::log!("passed global check");

        // If there is ALSO a password per use, check that as well.
        if let Some(pw) = &key_info.pw_per_use {
            let actual_pass = pw.get(cur_use).unwrap_or(hashed.clone());
            near_sdk::log!("hashed password: {:?}", hashed);
            near_sdk::log!(
                "actualPass password: {:?} cur use: {}",
                actual_pass,
                cur_use
            );

            if actual_pass != hashed {
                let used_gas = env::used_gas();

                let amount_to_decrement =
                    (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                near_sdk::log!(
                    "Incorrect password. Decrementing allowance by {}. Used GAS: {}",
                    amount_to_decrement,
                    used_gas.0
                );

                key_info.allowance -= amount_to_decrement;
                near_sdk::log!("Allowance is now {}", key_info.allowance);
                drop.key_info_by_token_id.insert(&token_id, &key_info);
                self.drop_for_id.insert(&drop_id, &drop);
                return false;
            }
        }

        near_sdk::log!("passed local check");

        // Otherwise return true
        true
    }

    /// Internal function to assert that the predecessor is the contract owner
    pub(crate) fn assert_claim_timestamps(
        &mut self,
        drop_id: DropId,
        drop: &mut Drop,
        key_info: &mut KeyInfo,
        token_id: &String,
    ) -> bool {
        if let Some(time_data) = drop.config.as_ref().and_then(|c| c.time.as_ref()) {
            // Ensure enough time has passed if a start timestamp was specified in the config.
            let current_timestamp = env::block_timestamp();

            let desired_start_timestamp = time_data.start.unwrap_or(0);
            if current_timestamp < desired_start_timestamp {
                let used_gas = env::used_gas();
    
                let amount_to_decrement =
                    (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                near_sdk::log!("Drop isn't claimable until {}. Current timestamp is {}. Decrementing allowance by {}. Used GAS: {}", desired_start_timestamp, current_timestamp, amount_to_decrement, used_gas.0);
    
                key_info.allowance -= amount_to_decrement;
                near_sdk::log!("Allowance is now {}", key_info.allowance);
                drop.key_info_by_token_id.insert(&token_id, &key_info);
                self.drop_for_id.insert(&drop_id, &drop);
                return false;
            }

            // Ensure the end timestamp hasn't passed and the key is still usable
            let desired_end_timestamp = time_data.end.unwrap_or(u64::MAX);
            if current_timestamp > desired_end_timestamp {
                let used_gas = env::used_gas();

                let amount_to_decrement =
                    (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                near_sdk::log!("Drop claim period passed. Claimable up until {}. Current timestamp is {}. Decrementing allowance by {}. Used GAS: {}", desired_end_timestamp, current_timestamp, amount_to_decrement, used_gas.0);

                key_info.allowance -= amount_to_decrement;
                near_sdk::log!("Allowance is now {}", key_info.allowance);
                drop.key_info_by_token_id.insert(&token_id, &key_info);
                self.drop_for_id.insert(&drop_id, &drop);
                return false;
            }

            // Ensure the key is within the throttle if specified
            if let Some(throttle) = time_data.throttle {
                near_sdk::log!(
                    "Current timestamp {} last used: {} subs: {} throttle: {}",
                    current_timestamp,
                    key_info.last_used,
                    current_timestamp - key_info.last_used,
                    throttle
                );

                if (current_timestamp - key_info.last_used) < throttle {
                    let used_gas = env::used_gas();

                    let amount_to_decrement =
                        (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                    near_sdk::log!("Not enough time has passed since the key was last used. Decrementing allowance by {}. Used GAS: {}", amount_to_decrement, used_gas.0);

                    key_info.allowance -= amount_to_decrement;
                    near_sdk::log!("Allowance is now {}", key_info.allowance);
                    drop.key_info_by_token_id.insert(&token_id, &key_info);
                    self.drop_for_id.insert(&drop_id, &drop);
                    return false;
                }

                near_sdk::log!("Enough time has passed for key to be used. Setting last used to current timestamp {}", current_timestamp);
                key_info.last_used = current_timestamp;
            }


            // Ensure the key is within the claim interval if specified
            if let Some(interval) = time_data.interval {
                let start_timestamp = time_data.start.unwrap();
                let total_num_uses = (env::block_timestamp() - start_timestamp) / interval;
                let uses_per_key = drop
                    .config
                    .as_ref()
                    .and_then(|c| c.uses_per_key)
                    .unwrap_or(0);
                let uses_left = total_num_uses + key_info.remaining_uses - uses_per_key;

                near_sdk::log!(
                    "Current timestamp {} start timestamp: {} claim interval: {} total num uses: {} total uses per key: {} remaining uses: {} num remaining uses: {}",
                    current_timestamp,
                    start_timestamp,
                    interval,
                    total_num_uses,
                    uses_per_key,
                    key_info.remaining_uses,
                    uses_left
                );

                if uses_left < 1 {
                    let used_gas = env::used_gas();

                    let amount_to_decrement =
                        (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
                    near_sdk::log!("Not enough time has passed before the key can be used. Decrementing allowance by {}. Used GAS: {}", amount_to_decrement, used_gas.0);

                    key_info.allowance -= amount_to_decrement;
                    near_sdk::log!("Allowance is now {}", key_info.allowance);
                    drop.key_info_by_token_id.insert(&token_id, &key_info);
                    self.drop_for_id.insert(&drop_id, &drop);
                    return false;
                }
            }

        }

        true
    }

    /// Internal function to register Keypom on a given FT contract
    pub(crate) fn internal_register_ft_contract(
        &mut self,
        ft_contract_id: &AccountId,
        storage_required: u128,
        account_to_refund: &AccountId,
        refund_balance: bool,
    ) {
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
            near_sdk::log!(
                "FT contract already registered. Refunding user balance for {}. Balance is now {}",
                yocto_to_near(storage_required),
                yocto_to_near(cur_user_bal)
            );
            self.user_balances.insert(account_to_refund, &cur_user_bal);
            return;
        }

        near_sdk::log!(
            "FT contract already registered. Transferring user for: {}",
            yocto_to_near(storage_required)
        );
        Promise::new(account_to_refund.clone()).transfer(storage_required);
    }

    /// Internal function to force remove a drop from the contract's state
    pub(crate) fn internal_remove_drop(
        &mut self,
        drop_id: &u128,
        public_keys: Vec<PublicKey>,
    ) -> AccountId {
        // Remove the drop
        let mut drop = self.drop_for_id.remove(drop_id).expect("drop not found");

        // Loop through public keys and remove all the keys and remove the key / passwrds per key
        for pk in &public_keys {
            let token_id = self.token_id_by_pk.remove(pk).unwrap();
            // Attempt to remove the public key. panic if it didn't exist
            let key_info = drop.key_info_by_token_id.remove(&token_id).expect("public key must be in drop");
            if let Some(mut k) = key_info.pw_per_use {
                k.clear();
            }
            if let Some(owner) = key_info.owner_id {
                self.internal_remove_token_from_owner(&owner, &token_id);
            }
        }
        assert!(drop.key_info_by_token_id.is_empty(), "drop not empty");
        //drop.pks.clear();

        let owner_id = drop.owner_id.clone();

        // Remove the drop ID from the funder's list
        self.internal_remove_drop_for_funder(&drop.owner_id, &drop_id);

        // Return the owner ID
        owner_id
    }

    /// Used to calculate the base allowance needed given attached GAS
    pub(crate) fn calculate_base_allowance(&self, attached_gas: Gas) -> u128 {
        let prepaid: u64 = attached_gas.0 + GAS_PER_CCC.0;

        // Get the number of CCCs you can make with the attached GAS
        // 5 TGas GAS_PER_CCC
        let calls_with_gas = (prepaid / GAS_PER_CCC.0) as f32;
        // Get the constant used to pessimistically calculate the required allowance
        let pow_outcome = 1.03_f32.powf(calls_with_gas);

        // Get the required GAS based on the calculated constant
        // 2.5 TGas receipt cost
        let required_allowance = ((prepaid + RECEIPT_GAS_COST.0) as f32 * pow_outcome
            + RECEIPT_GAS_COST.0 as f32) as u128
            * self.yocto_per_gas;
        near_sdk::log!(
            "{} calls with {} attached GAS. Pow outcome: {}. Required Allowance: {}",
            calls_with_gas,
            prepaid,
            pow_outcome,
            required_allowance
        );

        required_allowance
    }

    /// Add a drop ID to the set of drops a funder has
    pub(crate) fn internal_add_drop_to_funder(&mut self, account_id: &AccountId, drop_id: &DropId) {
        //get the set of drops for the given account
        let mut drop_set = self.drop_ids_for_funder.get(account_id).unwrap_or_else(|| {
            //if the account doesn't have any drops, we create a new unordered set
            UnorderedSet::new(StorageKey::DropIdsForFunderInner {
                //we get a new unique prefix for the collection
                account_id_hash: hash_account_id(&account_id.to_string()),
            })
        });

        //we insert the drop ID into the set
        drop_set.insert(drop_id);

        //we insert that set for the given account ID.
        self.drop_ids_for_funder.insert(account_id, &drop_set);
    }

    //remove a drop ID for a funder (internal method_name and can't be called directly via CLI).
    pub(crate) fn internal_remove_drop_for_funder(
        &mut self,
        account_id: &AccountId,
        drop_id: &DropId,
    ) {
        //we get the set of drop IDs that the funder has
        let mut drop_set = self
            .drop_ids_for_funder
            .get(account_id)
            //if there is no set of drops for the owner, we panic with the following message:
            .expect("No Drops found for the funder");

        //we remove the the drop ID from  the set of drops
        drop_set.remove(drop_id);

        //if the set is now empty, we remove the funder from the drop_ids_for_funder collection
        if drop_set.is_empty() {
            self.drop_ids_for_funder.remove(account_id);
        } else {
            //if the key set is not empty, we simply insert it back for the funder ID.
            self.drop_ids_for_funder.insert(account_id, &drop_set);
        }
    }

    /// Internal function for executing the callback code either straight up or using `.then` for a passed in promise
    pub(crate) fn internal_execute(
        &mut self,
        drop_data: Drop,
        drop_id: DropId,
        cur_key_id: u64,
        remaining_uses: u64,
        account_id: AccountId,
        storage_freed: u128,
        token_id: Option<String>,
        auto_withdraw: bool,
        fc_args: Option<Vec<Option<String>>>,
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
            DropType::fc(data) => {
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
                    // ID for the current key
                    cur_key_id,
                    // How many uses are remaining on the current key
                    remaining_uses,
                    // Maximum number of uses
                    drop_data.config.and_then(|c| c.uses_per_key).unwrap_or(1),
                    // Any user provided args
                    fc_args,
                    // Is it an auto withdraw case
                    auto_withdraw,
                ));
            }
            DropType::nft(data) => {
                resolve_promise_or_call!(on_claim_nft(
                    // Account ID that claimed the linkdrop
                    account_id,
                    // Account ID that funded the linkdrop
                    drop_data.owner_id.clone(),
                    // Balance associated with the linkdrop
                    U128(drop_data.deposit_per_use),
                    // How much storage was freed when the key was claimed
                    storage_freed,
                    // Sender of the NFT
                    data.sender_id.unwrap_or(drop_data.owner_id),
                    // Contract where the NFT is stored
                    data.contract_id,
                    // Token ID for the NFT
                    token_id.expect("no token ID found"),
                    // Is it an auto withdraw case
                    auto_withdraw,
                ));
            }
            DropType::ft(data) => {
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
            DropType::simple(_) => {
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

    /// Check whether or not a given string is a valid account ID or public key. Soft panic if it isn't.
    pub(crate) fn assert_valid_args(
        &mut self, 
        account_id: String,
        pub_key: Option<String>,
        drop_id: DropId,
        drop: &mut Drop,
        key_info: &mut KeyInfo,
        token_id: &String
    ) -> bool {
        let account_id_valid = AccountId::try_from(account_id).is_ok();
        let mut pub_key_valid = true;

        if let Some(key) = pub_key {
            pub_key_valid = PublicKey::from_str(key.as_str()).is_ok();
        }

        if !account_id_valid || !pub_key_valid {
            let used_gas = env::used_gas();

            let amount_to_decrement =
                (used_gas.0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
            near_sdk::log!(
                "Invalid Account Id Passed In. Decrementing allowance by {} Used GAS: {}",
                amount_to_decrement,
                used_gas.0
            );

            key_info.allowance -= amount_to_decrement;
            near_sdk::log!("Allowance is now {}", key_info.allowance);
            drop.key_info_by_token_id.insert(&token_id, &key_info);
            self.drop_for_id.insert(&drop_id, &drop);

            return false
        }

        true
    }
}
