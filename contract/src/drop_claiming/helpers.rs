use near_sdk::{PromiseResult, ext_contract, env::sha256};

use crate::*;

/// Interface for account creation
#[ext_contract(ext_account_creation)]
trait ExtAccountCreation {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}

#[near_bindgen]
impl Keypom {
    /// Ensure re-entry protection and decrement remaining uses on a key
    /// Returns the drop ID that the key is associated with
    pub(crate) fn before_claim_logic(
        &mut self, 
        event_logs: &mut Vec<EventLog>, 
        receiver_id: &AccountId, 
        new_public_key: Option<&PublicKey>,
        password: Option<String>
    ) -> BeforeClaimData {
        let signer_pk = env::signer_account_pk();

        // Get the key info and decrement its remaining uses.
        // If there are zero remaining uses, break the connection between
        // The public key and token ID that way the key can't be used anymore.
        // This state will be written in the current block so prevents re-entrancy attacks.
        // We shouldn't just remove the public key mapping off the bat because there might be
        // Multi use keys that submit multiple txns in the same block. Only delete if empty.
        let token_id = self
            .token_id_by_pk
            .get(&signer_pk)
            .expect("No drop ID found for PK");
        
        let (drop_id, _) = parse_token_id(&token_id).unwrap();
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let InternalAssetDataForUses { uses: _, config: use_config, assets_metadata } = get_asset_data_for_specific_use(&drop.asset_data_for_uses, &cur_key_use);
        
        assert_pre_claim_conditions(
            &key_info,
            &use_config,
            &password,
            &cur_key_use,
            &drop.max_key_uses,
            new_public_key.is_some()
        );
        
        // Tally up all the gas for the assets
        let mut required_asset_gas = Gas(0);
        let mut assets = Vec::new();
        for metadata in assets_metadata {
            let internal_asset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            
            // For claim events
            assets.push(internal_asset.to_external_asset(&metadata.tokens_per_use));
            required_asset_gas += internal_asset.get_required_gas();
        }
        
        key_info.remaining_uses -= 1;
        key_info.last_claimed = env::block_timestamp();
        if key_info.remaining_uses == 0 {
            // Delete everything except the token ID -> key info mapping since we need the key info in callbacks
            if let Some(owner) = key_info.owner_id.as_ref() {
                self.internal_remove_token_from_owner(owner, &token_id);
            }

            self.token_id_by_pk.remove(&signer_pk);
            Promise::new(env::current_account_id()).delete_key(signer_pk.clone());
            
            event_logs.push(EventLog {
                standard: NFT_STANDARD_NAME.to_string(),
                version: NFT_METADATA_SPEC.to_string(),
                event: EventLogVariant::NftBurn(vec![NftBurnLog {
                    owner_id: key_info.owner_id.as_ref().unwrap_or(&env::current_account_id()).to_string(),
                    token_ids: vec![token_id.to_string()],
                    authorized_id: None,
                    memo: None,
                }]),
            });
            event_logs.push(EventLog {
                standard: KEYPOM_STANDARD_NAME.to_string(),
                version: KEYPOM_STANDARD_VERSION.to_string(),
                event: EventLogVariant::DeleteKey(vec![AddOrDeleteKeyLog {
                    drop_id: drop_id.to_string(),
                    public_key: (&signer_pk).into()
                }]),
            });
        }

        drop.key_info_by_token_id.insert(&token_id, &key_info);
        self.drop_by_id.insert(&drop_id, &drop);

        // Log either CAAC or claim events depending on whether or not a new public key was provided
        if let Some(pk) = new_public_key {
            event_logs.push(EventLog {
                standard: KEYPOM_STANDARD_NAME.to_string(),
                version: KEYPOM_STANDARD_VERSION.to_string(),
                event: EventLogVariant::CreateAccountAndClaim(CreateAccountAndClaimLog { 
                    new_account_id: receiver_id.to_string(), 
                    new_public_key: pk.into(), 
                    public_key: (&signer_pk).into(), 
                    drop_id, 
                    assets
                }),
            })
        } else {
            event_logs.push(EventLog {
                standard: KEYPOM_STANDARD_NAME.to_string(),
                version: KEYPOM_STANDARD_VERSION.to_string(),
                event: EventLogVariant::Claim(ClaimLog { 
                    account_id: receiver_id.to_string(), 
                    public_key: (&signer_pk).into(), 
                    drop_id, 
                    assets
                }),
            })
        };

        // For CAAC, there needs to be a root for all accounts. By default, this is the contract's global root account (i.e `near` or `testnet`) but if otherwise specified in the use or drop config, it will be that.
        let root_account_id = use_config.as_ref().and_then(|c| c.root_account_id.clone()).unwrap_or(self.root_account.clone());
        let account_creation_keypom_args = use_config.as_ref().and_then(|c| c.account_creation_keypom_args.clone());
        BeforeClaimData {
            token_id,
            required_asset_gas,
            root_account_id,
            account_creation_keypom_args,
        }
    }

    /// Internal function that loops through all assets for the given use and claims them.
    /// Should be executed in both `claim` or `create_account_and_claim`
    /// Once all assets are claimed, a cross-contract call is fired to `on_assets_claimed`
    pub(crate) fn internal_claim_assets(&mut self, token_id: TokenId, receiver_id: AccountId, fc_args: UserProvidedFCArgs) -> PromiseOrValue<bool> {
        let (drop_id, key_id) = parse_token_id(&token_id).unwrap();

        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        // The uses were decremented before the claim, so we need to increment them back to get what use should be refunded
        let cur_key_use = get_key_cur_use(&drop, &key_info) - 1;
        let InternalAssetDataForUses { uses: _, config: _, assets_metadata } = get_asset_data_for_specific_use(&drop.asset_data_for_uses, &cur_key_use);
        
        //let promises;
        let mut promises = Vec::new();
        let mut token_ids_transferred = Vec::new();
        let mut fc_arg_idx = 0;
        for metadata in assets_metadata {
            let mut asset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found").clone();
            
            // We need to keep track of all the NFT token IDs in order to potentially perform refunds
            if let InternalAsset::nft(data) = &asset {
                token_ids_transferred.push(data.get_next_token_id());
            } else {
                token_ids_transferred.push(None);
            }

            // Try to get the fc args for the asset. If the length of the fc_args outer vector is not the same as the number of FC assets
            // Meaning that the user didn't specify fc args (even as none) for each asset, just default it to None once it gets out of range
            let fc_args_for_asset = fc_args.as_ref().and_then(|a| a.get(fc_arg_idx).cloned()).unwrap_or(None);
            
            // Some cases may result in no promise index (i.e not enough balance)
            promises.push(asset.claim_asset(
                &receiver_id, 
                &metadata.tokens_per_use.map(|x| x.into()),
                fc_args_for_asset,
                drop_id.clone(),
                key_id.to_string(),
                drop.funder_id.clone()
            ));

            // Increment the number of fc args we've seen
            if let InternalAsset::fc(_) = asset {
                fc_arg_idx += 1;
            }
            
            drop.asset_by_id.insert(metadata.asset_id, asset.clone());
        }

        // Put the modified drop back in storage
        self.drop_by_id.insert(&drop_id, &drop);

        if let Some(resolve) = promises.into_iter().reduce(|a, b| a.and(b)).expect("empty promises") {
            PromiseOrValue::Promise(resolve.then(
                Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_RESOLVE_ASSET_CLAIM)
                    .with_unused_gas_weight(1)
                    .on_assets_claimed(
                        token_id,
                        token_ids_transferred
                    )
            ))
        } else {
            self.on_assets_claimed(token_id, token_ids_transferred)
        }
    }

    /// Check if key is empty and perform cleanup if it is
    /// This includes removing the drop if it now has 0 keys and no refundable assets
    pub(crate) fn handle_key_cleanup(
        &mut self,
        drop: &mut InternalDrop,
        key_info: &InternalKeyInfo,
        token_id: &TokenId,
        drop_id: &DropId,
        drop_assets_withdrawn: bool,
        initial_storage: u64
    ) {
        let mut event_logs = vec![];
        // Now that the callback is finished, we can remove the key info from the drop
        // Since no other functions need the key information
        if key_info.remaining_uses == 0 {
            drop.key_info_by_token_id.remove(&token_id).expect("Key not found");
    
            let should_delete_on_empty = drop.config.as_ref().and_then(|c| c.delete_empty_drop).unwrap_or(true);

            // Now that we've removed the key info, check if the drop is empty
            // Otherwise, re-insert the drop into state
            if drop.key_info_by_token_id.is_empty() && drop_assets_withdrawn && should_delete_on_empty {
                near_sdk::log!("Drop with ID: {} is now empty. Deleting.", drop_id);
                // Remove the drop from storage and clear the maps inside of it
                self.drop_by_id.remove(&drop_id);
                self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
                internal_clear_drop_storage(drop, &mut event_logs, &drop_id);
            } else {
                near_sdk::log!("Drop with ID: {} is not empty. Re-inserting. Does have assets? {} Config specified to delete: {}", drop_id, !drop_assets_withdrawn, should_delete_on_empty);
                // Put the modified drop back in storage
                self.drop_by_id.insert(&drop_id, &drop);
            }
        }
    
        let final_storage = env::storage_usage();
        // Some storage was freed so we should refund the user's balance
        if final_storage < initial_storage {
            let storage_cost = (initial_storage - final_storage) as u128 * env::storage_byte_cost();
            self.internal_modify_user_balance(&drop.funder_id, storage_cost, false);
        }

        log_events(event_logs);
    }
}

/// Returns whether or not the account was successfully created when firing the cross contract call to `create_account`
pub(crate) fn was_account_created() -> bool {
    if let PromiseResult::Successful(value) = env::promise_result(0) {
        if let Ok(account_created) = near_sdk::serde_json::from_slice::<bool>(&value) {
            //if we need don't need to return the token, we simply return true meaning everything went fine
            near_sdk::log!("create_account call returned {}", account_created);
            return account_created;
        }
    }

    false
}

/// Internal function to perform all the pre-claim checks such as passwords, configs etc.
pub(crate) fn assert_pre_claim_conditions(
    key_info: &InternalKeyInfo,
    use_config: &Option<UseConfig>,
    user_password: &Option<String>,
    cur_key_use: &UseNumber,
    max_uses_per_key: &UseNumber,
    creating_account: bool
) {
    // Ensure that claim and create_account_and_claim are only called based on the key / drop's config
    if let Some(perm) = use_config.as_ref().and_then(|c| c.permissions.as_ref()) {
        match perm {
            ClaimPermissions::claim => {
                require!(creating_account == false, "Cannot call `create_account_and_claim` when key permission is set to only claim")
            },
            ClaimPermissions::create_account_and_claim => {
                require!(creating_account == true, "Cannot call `claim` when key permission is set to only create_account_and_claim")
            },
        }
    }

    // Ensure any timestamps in the configs have been fulfilled
    assert_claim_timestamps(use_config, key_info, &String::from(&env::signer_account_pk()), max_uses_per_key);

    // If there is some password for the current key use, assert that it matches the one provided
    if let Some(pw_by_use) = &key_info.pw_by_use {
        if let Some(pw) = pw_by_use.get(cur_key_use) {
            assert_key_password(user_password, pw.clone());
        }
    }
}

/// Internal function to assert that the password for claim matches the one in the key info
pub(crate) fn assert_key_password(
    user_password: &Option<String>,
    expected_password: Vec<u8>
) {
    let hashed_user_pw = sha256(&user_password.as_ref().and_then(|f| hex::decode(f).ok()).expect("Password expected."));

    require!(hashed_user_pw == expected_password, format!("User provided password: {:?} does not match expected password: {:?}", hashed_user_pw, expected_password));
}

/// Internal function to assert that the predecessor is the contract owner
pub(crate) fn assert_claim_timestamps(
    use_config: &Option<UseConfig>,
    key_info: &InternalKeyInfo,
    signer_pk: &String,
    max_uses_per_key: &UseNumber,
) {
    let time_config = use_config.as_ref().and_then(|c| c.time.as_ref());
    
    if let Some(time_data) = time_config {
        // Ensure enough time has passed if a start timestamp was specified in the config.
        let current_timestamp = env::block_timestamp();

        let desired_start_timestamp = time_data.start.unwrap_or(0);
        require!(current_timestamp >= desired_start_timestamp, format!("Key {} isn't claimable until {}. Current timestamp {}", signer_pk, desired_start_timestamp, current_timestamp));

        // Ensure the end timestamp hasn't passed and the key is still usable
        let desired_end_timestamp = time_data.end.unwrap_or(u64::MAX);
        require!(current_timestamp <= desired_end_timestamp, format!("Key {} is no longer claimable. It was claimable up until {}. Current timestamp {}", signer_pk, desired_end_timestamp, current_timestamp));

        let throttle = time_data.throttle.unwrap_or(u64::MAX);
        require!((current_timestamp - key_info.last_claimed) >= throttle, format!("Key {} was used too recently. It must be used ever {}. Time since last use {}", signer_pk, throttle, current_timestamp - key_info.last_claimed));


        // Ensure the key is within the claim interval if specified
        if let Some(interval) = time_data.interval {
            let start_timestamp = time_data.start.unwrap();

            // At this moment, what is the maximum number of uses that COULD have been possibly claimed
            let total_possible_claims = (env::block_timestamp() - start_timestamp) / interval;
            
            // How many claims given the interval and remaining uses does this key have left?
            // Take the current total claims possible, and subtract the number of uses the key has left
            // Example: If the interval is 1 day, and 4 days have passed, the key has 4 total claimable uses.
            // If the key has already been claimed 3/4 times, then the key has 1 remaining claimable uses.
            let claims_so_far = max_uses_per_key - key_info.remaining_uses;
            let num_claimable_uses = total_possible_claims - claims_so_far as u64;

            require!(num_claimable_uses > 0, format!("Key {} has been claimed {} times. Given the interval {} and starting timestamp of {}, there are {} claims available.", signer_pk, claims_so_far, interval, start_timestamp, num_claimable_uses));
        }
    }
}