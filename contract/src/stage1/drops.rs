use crate::*;

use near_sdk::{
    collections::{LazyOption, Vector},
    require, Balance,
};

#[near_bindgen]
impl Keypom {
    /*
        user has created a bunch of keypairs and passed in the public keys and attached some attached_deposit.
        this will store the account data and allow that keys to call claim and create_account_and_claim
        on this contract.

        The balance is the amount of $NEAR the sender wants each linkdrop to contain.
    */
    #[payable]
    pub fn create_drop(
        &mut self,
        // Public keys to add when creating the drop (can be empty)
        public_keys: Option<Vec<PublicKey>>,
        // How much $NEAR should be transferred everytime a key is used? Can be 0.
        deposit_per_use: U128,

        // Overload the specific drop ID
        drop_id: Option<DropIdJson>,
        // Configure behaviors for the drop
        config: Option<DropConfig>,
        // Give the drop some metadata (simple string)
        metadata: Option<DropMetadata>,

        // Mutually Exclusive. Use-case specific configurations
        simple: Option<SimpleData>,
        ft: Option<JsonFTData>,
        nft: Option<JsonNFTData>,
        fc: Option<FCData>,

        // Passwords for the keys
        passwords_per_use: Option<Vec<Option<Vec<JsonPasswordForUse>>>>,
        passwords_per_key: Option<Vec<Option<String>>>,
    ) -> Option<DropIdJson> {
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();

        let mut actual_drop_id = self.next_drop_id;

        if let Some(id) = drop_id {
            require!(
                !self.drop_for_id.contains_key(&id.into()),
                "Drop ID already exists"
            );
            require!(id.0 >= MIN_DROP_ID_PASSED_IN, "drop ID too small");
            actual_drop_id = id.0;
        } else {
            // Increment the drop ID nonce only if no drop Id was passed in
            self.next_drop_id += 1;
        }

        // Ensure the user has only specified one type of callback data
        let num_drop_types =
            ft.is_some() as u8 + nft.is_some() as u8 + fc.is_some() as u8 + simple.is_some() as u8;
        require!(
            num_drop_types <= 1,
            "You cannot specify more than one drop type data"
        );

        // Warn if the balance for each drop is less than the minimum
        if deposit_per_use.0 < NEW_ACCOUNT_BASE {
            near_sdk::log!(
                "Warning: Balance is less than absolute minimum for creating an account: {}",
                NEW_ACCOUNT_BASE
            );
        }

        // Ensure the user has specified a valid drop config
        assert_valid_drop_config(&config);

        // Funder is the predecessor
        let owner_id = env::predecessor_account_id();
        let keys_to_iter = public_keys.unwrap_or_default();
        let len = keys_to_iter.len() as u128;
        // Get the number of uses per key to dictate what key usage data we should put in the map
        let num_uses_per_key = config.clone().and_then(|c| c.uses_per_key).unwrap_or(1);

        // Get the current balance of the funder.
        let mut current_user_balance = self.user_balances.get(&owner_id).unwrap_or(0);
        let near_attached = env::attached_deposit();
        // Add the attached deposit to their balance
        current_user_balance += near_attached;
        self.user_balances.insert(&owner_id, &current_user_balance);

        let mut key_map: UnorderedMap<PublicKey, KeyInfo> =
            UnorderedMap::new(StorageKey::PksForDrop {
                // We get a new unique prefix for the collection
                account_id_hash: hash_account_id(&format!("{}{}", actual_drop_id, owner_id)),
            });

        // Decide what methods the access keys can call
        let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
        if let Some(perms) = config.clone().and_then(|c| c.usage).and_then(|u| u.permissions) {
            match perms {
                // If we have a config, use the config to determine what methods the access keys can call
                ClaimPermissions::claim => {
                    access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
                }
                ClaimPermissions::create_account_and_claim => {
                    access_key_method_names = ACCESS_KEY_CREATE_ACCOUNT_METHOD_NAME;
                }
            }
        }

        // Default the gas to attach to be the gas from the wallet. This will be used to calculate allowances.
        let mut gas_to_attach = ATTACHED_GAS_FROM_WALLET;
        // Depending on the FC Data, set the Gas to attach and the access key method_name names
        if let Some(gas) = fc
            .clone()
            .and_then(|d| d.config.and_then(|c| c.attached_gas))
        {
            require!(
                deposit_per_use.0 == 0,
                "cannot specify gas to attach and have a balance in the linkdrop"
            );
            require!(
                gas <= ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE,
                &format!(
                    "cannot attach more than {:?} GAS.",
                    ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE
                )
            );
            gas_to_attach = gas + GAS_OFFSET_IF_FC_EXECUTE;
            access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
        }

        // Calculate the base allowance to attach
        let calculated_base_allowance = self.calculate_base_allowance(gas_to_attach);
        // The actual allowance is the base * number of uses per key since each claim can potentially use the max pessimistic GAS.
        let actual_allowance = calculated_base_allowance * num_uses_per_key as u128;

        if passwords_per_use.is_some() {
            require!(len <= 50, "Cannot add 50 keys at once with passwords");
        }
        require!(
            passwords_per_use
                .clone()
                .map(|f| f.len() as u128)
                .unwrap_or(len)
                == len,
            "Passwords per use must be equal to the number of public keys"
        );
        require!(
            passwords_per_key
                .clone()
                .map(|f| f.len() as u128)
                .unwrap_or(len)
                == len,
            "Passwords per key must be equal to the number of public keys"
        );
        // Loop through and add each drop ID to the public keys. Also populate the key set.
        let mut next_key_id: u64 = 0;
        for pk in &keys_to_iter {
            let pw_per_key = passwords_per_key
                .clone()
                .and_then(|f| f[next_key_id as usize].clone())
                .map(|h| hex::decode(h).unwrap());

            let mut pw_per_use = None;
            // If we have passwords for this specific key, add them to the key info
            if let Some(pws) = passwords_per_use
                .as_ref()
                .and_then(|p| p[next_key_id as usize].as_ref())
            {
                let mut pw_map = UnorderedMap::new(StorageKey::PasswordsPerUse {
                    // We get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&format!(
                        "pws-{}{}{}",
                        next_key_id, actual_drop_id, owner_id
                    )),
                });

                // Loop through each password and add it to the lookup map
                for pw in pws {
                    require!(
                        pw.key_use <= num_uses_per_key,
                        "claim out of range for password"
                    );
                    pw_map.insert(&pw.key_use, &hex::decode(pw.pw.clone()).unwrap());
                }

                pw_per_use = Some(pw_map);
            }

            require!(
                pw_per_use.is_some() as u8 + pw_per_key.is_some() as u8 <= 1,
                "You cannot specify both local and global passwords for a key"
            );

            key_map.insert(
                pk,
                &KeyInfo {
                    remaining_uses: num_uses_per_key,
                    last_used: 0, // Set to 0 since this will make the key always claimable.
                    allowance: actual_allowance,
                    key_id: next_key_id,
                    pw_per_use,
                    pw_per_key,
                },
            );
            require!(
                self.drop_id_for_pk.insert(pk, &actual_drop_id).is_none(),
                "Keys cannot belong to another drop"
            );
            next_key_id += 1;
        }

        // Add this drop ID to the funder's set of drops
        self.internal_add_drop_to_funder(&env::predecessor_account_id(), &actual_drop_id);

        // Create drop object
        let mut drop = Drop {
            owner_id: env::predecessor_account_id(),
            deposit_per_use: deposit_per_use.0,
            pks: key_map,
            // Default to simple but will overwrite if not
            drop_type: DropType::simple(
                SimpleData {
                    lazy_register: None
                }
            ),
            config: config.clone(),
            registered_uses: num_uses_per_key * len as u64,
            required_gas: gas_to_attach,
            metadata: LazyOption::new(
                StorageKey::DropMetadata {
                    // We get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&format!(
                        "metadata-{}{}",
                        actual_drop_id, owner_id
                    )),
                },
                metadata.as_ref(),
            ),
            next_key_id,
        };

        let lazy_register = simple.clone().and_then(|s| s.lazy_register).unwrap_or(false);
        // Keep track of the total attached_deposit required for the FC data (depending on None and Some cases)
        let mut deposit_required_for_fc_deposits = 0;
        // Keep track of the number of none FCs so we don't charge the user
        let mut num_none_fcs = 0;
        let mut was_ft_registered = false;

        // If NFT data was provided, we need to build the set of token IDs and cast the config to actual NFT data
        if let Some(data) = nft {
            require!(lazy_register == false, "lazy_register is reserved for simple drops only");

            let JsonNFTData {
                sender_id,
                contract_id,
            } = data;

            // Create the token ID vector
            let token_ids = Vector::new(StorageKey::TokenIdsForDrop {
                //we get a new unique prefix for the collection
                account_id_hash: hash_account_id(&format!("nft-{}{}", actual_drop_id, owner_id)),
            });

            // Create the NFT data
            let actual_nft = NFTData {
                sender_id,
                contract_id,
                token_ids,
            };

            // The number of uses is 0 until NFTs are sent to the contract
            drop.registered_uses = 0;
            drop.drop_type = DropType::nft(actual_nft);

            // Add the drop with the empty token IDs
            self.drop_for_id.insert(&actual_drop_id, &drop);
        } else if let Some(data) = ft.clone() {
            require!(lazy_register == false, "lazy_register is reserved for simple drops only");

            // If FT Data was provided, we need to cast the FT Config to actual FT data and insert into the drop type
            let JsonFTData {
                sender_id,
                contract_id,
                balance_per_use,
            } = data;

            // Create the NFT data
            let actual_ft = FTData {
                contract_id: contract_id.clone(),
                sender_id,
                balance_per_use,
                ft_storage: U128(u128::MAX),
            };

            // Temporarily add the FT contract. If it was already registered, don't remove but if it was, remove now.
            // This is to measure the storage cost of adding the contract.
            let clean_insert = self.registered_ft_contracts.insert(&contract_id);
            was_ft_registered = !clean_insert;
            near_sdk::log!("was_ft_registered: {}", was_ft_registered);

            // The number of uses is 0 until FTs are sent to the contract
            drop.registered_uses = 0;
            drop.drop_type = DropType::ft(actual_ft);

            // Add the drop with the empty token IDs
            self.drop_for_id.insert(&actual_drop_id, &drop);
        } else if let Some(data) = fc.clone() {
            require!(lazy_register == false, "lazy_register is reserved for simple drops only");

            drop.drop_type = DropType::fc(data.clone());

            // Ensure proper method data is passed in
            let num_method_data = data.clone().methods.len() as u64;
            // If there's 1 claim, there should be 1 method data defined
            if num_uses_per_key == 1 {
                require!(
                    num_method_data == 1,
                    "Cannot have more Method Data than the number of uses per key"
                );
            // If there's more than 1 method data defined and the number of uses per key more than 1, the number of methods should equal the number of uses per key
            } else if num_method_data > 1 {
                require!(
                    num_method_data == num_uses_per_key,
                    "Number of FCs must match number of uses per key if more than 1 is specified"
                );
            }

            // If there's one method data specified and more than 1 claim per key, that data is to be used
            // For all the uses. In this case, we need to tally all the deposits for each method in all method data.
            if num_uses_per_key > 1 && num_method_data == 1 {
                let method_data = data
                    .methods
                    .iter()
                    .next()
                    .unwrap()
                    .clone()
                    .expect("cannot have a single none function call");

                // Keep track of the total attached deposit across all methods in the method data
                let mut attached_deposit = 0;
                // Iterate through each method in the method data and accumulate the attached deposit
                for method in method_data {
                    // Add the attached_deposit to the total deposit required
                    attached_deposit += method.attached_deposit.0;

                    // Ensure no malicious activity is going on
                    require!(
                        method.receiver_id != env::current_account_id(),
                        "Cannot invoke functions on keypom"
                    );
                    require!(
                        self.prohibited_fc_methods.contains(&method.method_name) == false,
                        "Cannot invoke a prohibited function call"
                    );
                }

                deposit_required_for_fc_deposits = num_uses_per_key as u128 * attached_deposit;
            // In the case where either there's 1 claim per key or the number of FCs is not 1,
            // We can simply loop through and manually get this data
            } else {
                for method_data in data.methods {
                    num_none_fcs += method_data.is_none() as u64;
                    // If the method is not None, we need to get the attached_deposit by looping through the method datas
                    if let Some(data) = method_data {
                        // Keep track of the total attached deposit across all methods in the method data
                        let mut attached_deposit = 0;
                        // Iterate through each method in the method data and accumulate the attached deposit
                        for method in data {
                            // Add the attached_deposit to the total deposit required
                            attached_deposit += method.attached_deposit.0;

                            // Ensure no malicious activity is going on
                            require!(
                                method.receiver_id != env::current_account_id(),
                                "Cannot invoke functions on keypom"
                            );
                            require!(
                                self.prohibited_fc_methods.contains(&method.method_name) == false,
                                "Cannot invoke a prohibited function call"
                            );
                        }

                        deposit_required_for_fc_deposits += attached_deposit;
                    }
                }
            }

            // Add the drop with the empty token IDs
            self.drop_for_id.insert(&actual_drop_id, &drop);
        } else {
            require!(
                deposit_per_use.0 > 0,
                "Cannot have a simple drop with zero balance"
            );

            // If the user wants to register keys later, they can.
            if lazy_register {
                drop.registered_uses = 0;
            }

            if let Some(simple_data) = simple {
                drop.drop_type = DropType::simple(simple_data);
            }

            // In simple case, we just insert the drop with whatever it was initialized with.
            self.drop_for_id.insert(&actual_drop_id, &drop);
        }

        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_required_storage =
            Balance::from(final_storage - initial_storage) * env::storage_byte_cost();
        near_sdk::log!("Total required storage Yocto {}", total_required_storage);

        /*
            Required attached_deposit consists of:
            - Fees
            - TOTAL Storage
            - Total access key allowance for EACH key
            - Access key storage for EACH key
            - Balance for each key * (number of uses - uses with None for FC Data)

            Optional:
            - FC attached_deposit for each key * num Some(data) uses
            - FT storage registration cost for each key * uses (calculated in resolve storage calculation function)
        */
        let fees = self
            .fees_per_user
            .get(&owner_id)
            .unwrap_or((self.drop_fee, self.key_fee));
        let drop_fee = fees.0;
        let key_fee = fees.1;
        let total_key_fee = key_fee * len;
        let total_allowance = actual_allowance * len;
        let total_access_key_storage = ACCESS_KEY_STORAGE * len;
        let total_deposits = (deposit_per_use.0 * (num_uses_per_key - num_none_fcs) as u128 * len) * !lazy_register as u128;
        let total_deposits_for_fc = deposit_required_for_fc_deposits * len;

        let required_deposit = drop_fee
            + total_required_storage
            + total_key_fee
            + total_allowance
            + total_access_key_storage
            + total_deposits
            + total_deposits_for_fc;

        near_sdk::log!(
            "Current balance: {}, 
            Required Deposit: {}, 
            total_required_storage: {},
            Drop Fee: {}, 
            Key Fee: {} Total Key Fee: {},
            allowance: {} total allowance: {},
            access key storage: {} total access key storage: {},
            deposits less none FCs: {} total deposits: {} lazy registration: {},
            deposits for FCs: {} total deposits for FCs: {},
            uses per key: {}
            None FCs: {},
            length: {}
            GAS to attach: {}",
            yocto_to_near(current_user_balance),
            yocto_to_near(required_deposit),
            yocto_to_near(total_required_storage),
            yocto_to_near(drop_fee),
            yocto_to_near(key_fee),
            yocto_to_near(total_key_fee),
            yocto_to_near(actual_allowance),
            yocto_to_near(total_allowance),
            yocto_to_near(ACCESS_KEY_STORAGE),
            yocto_to_near(total_access_key_storage),
            yocto_to_near(deposit_per_use.0 * (num_uses_per_key - num_none_fcs) as u128),
            yocto_to_near(total_deposits),
            lazy_register,
            yocto_to_near(deposit_required_for_fc_deposits),
            yocto_to_near(total_deposits_for_fc),
            num_uses_per_key,
            num_none_fcs,
            len,
            gas_to_attach.0
        );

        /*
            Ensure the attached attached_deposit can cover:
        */
        if current_user_balance < required_deposit {
            near_sdk::log!(
                "Not enough user balance. Found {} expected: {}",
                yocto_to_near(current_user_balance),
                yocto_to_near(required_deposit)
            );
            current_user_balance -= near_attached;

            // If they have a balance, insert it back into the map otherwise remove it
            if current_user_balance > 0 {
                self.user_balances.insert(&owner_id, &current_user_balance);
            } else {
                self.user_balances.remove(&owner_id);
            }

            // Refund the predecessor for their attached deposit if it's greater than 0
            if near_attached > 0 {
                Promise::new(env::predecessor_account_id()).transfer(near_attached);
            }

            // Remove the drop
            self.internal_remove_drop(&actual_drop_id, keys_to_iter);
            // Return early
            return None;
        }

        // Decrement the user's balance by the required attached_deposit and insert back into the map
        current_user_balance -= required_deposit;
        self.user_balances.insert(&owner_id, &current_user_balance);
        near_sdk::log!("New user balance {}", yocto_to_near(current_user_balance));

        // Increment our fees earned
        self.fees_collected += fees.0 + fees.1 * len;
        near_sdk::log!("Fees collected {}", yocto_to_near(fees.0 + fees.1 * len));

        let current_account_id = env::current_account_id();

        /*
            Only add the access keys if it's not a FT drop. If it is,
            keys will be added in the FT resolver
        */
        if ft.is_none() {
            // Create a new promise batch to create all the access keys
            let promise = env::promise_batch_create(&current_account_id);

            // Loop through each public key and create the access keys
            for pk in keys_to_iter.clone() {
                // Must assert in the loop so no access keys are made?
                env::promise_batch_action_add_key_with_function_call(
                    promise,
                    &pk,
                    0,
                    actual_allowance,
                    &current_account_id,
                    access_key_method_names,
                );
            }

            env::promise_return(promise);
        } else {
            let ft_contract = ft.unwrap().contract_id;
            // If the ft contract was NOT already registered, we should remove it from the set here and add it
            // Only if everything went well in the callback.
            if !was_ft_registered {
                near_sdk::log!("FT contract was not already registered. Removing from set");
                self.registered_ft_contracts.remove(&ft_contract);
            }

            /*
                Get the storage required by the FT contract and ensure the user has attached enough
                attached_deposit to cover the storage and perform refunds if they overpayed.
            */
            near_sdk::log!("Performing CCC to get storage from FT contract");
            ext_ft_contract::ext(ft_contract)
                // Call storage balance bounds with exactly this amount of GAS. No unspent GAS will be added on top.
                .with_static_gas(GAS_FOR_STORAGE_BALANCE_BOUNDS)
                .with_unused_gas_weight(0)
                .storage_balance_bounds()
                .then(
                    Self::ext(current_account_id)
                        // Resolve the promise with the min GAS. All unspent GAS will be added to this call.
                        .with_attached_deposit(near_attached)
                        .with_static_gas(MIN_GAS_FOR_RESOLVE_STORAGE_CHECK)
                        .resolve_storage_check(keys_to_iter, actual_drop_id, required_deposit),
                );
        }

        Some(U128(actual_drop_id))
    }

    /*
        Allows users to add to an existing drop.
        Only the funder can call this method_name
    */
    #[payable]
    pub fn add_keys(
        &mut self,
        // Public keys to add when creating the drop (can be empty)
        public_keys: Option<Vec<PublicKey>>,
        // Overload the specific drop ID
        drop_id: DropIdJson,

        // Passwords for the keys
        passwords_per_use: Option<Vec<Option<Vec<JsonPasswordForUse>>>>,
        passwords_per_key: Option<Vec<Option<String>>>,
    ) -> Option<DropIdJson> {
        let mut drop = self
            .drop_for_id
            .get(&drop_id.0)
            .expect("no drop found for ID");
        let config = &drop.config;
        let funder = &drop.owner_id;

        require!(
            funder == &env::predecessor_account_id(),
            "only funder can add to drops"
        );

        let keys_to_iter = public_keys.unwrap_or_default();
        let len = keys_to_iter.len() as u128;
        /*
            Add data to storage
        */
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();

        // Get the number of uses per key
        let num_uses_per_key = config.clone().and_then(|c| c.uses_per_key).unwrap_or(1);

        // get the existing key set and add new PKs
        let mut exiting_key_map = drop.pks;

        // Calculate the base allowance to attach
        let calculated_base_allowance = self.calculate_base_allowance(drop.required_gas);
        // The actual allowance is the base * number of uses per key since each claim can potentially use the max pessimistic GAS.
        let actual_allowance = calculated_base_allowance * num_uses_per_key as u128;

        if passwords_per_use.is_some() {
            require!(len <= 50, "Cannot add 50 keys at once with passwords");
        }
        require!(
            passwords_per_use
                .clone()
                .map(|f| f.len() as u128)
                .unwrap_or(len)
                == len,
            "Passwords per use must be less than or equal to the number of public keys"
        );
        require!(
            passwords_per_key
                .clone()
                .map(|f| f.len() as u128)
                .unwrap_or(len)
                == len,
            "Passwords per key must be equal to the number of public keys"
        );
        // Loop through and add each drop ID to the public keys. Also populate the key set.
        let mut next_key_id: u64 = drop.next_key_id;
        let mut idx = 0;
        for pk in &keys_to_iter {
            let pw_per_key = passwords_per_key
                .clone()
                .and_then(|f| f[idx as usize].clone())
                .map(|f| hex::decode(f).unwrap());

            let mut pw_per_use = None;
            // If we have passwords for this specific key, add them to the key info
            if let Some(pws) = passwords_per_use
                .as_ref()
                .and_then(|p| p[idx as usize].as_ref())
            {
                let mut pw_map = UnorderedMap::new(StorageKey::PasswordsPerUse {
                    // We get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&format!(
                        "pws-{}{}{}",
                        next_key_id, drop_id.0, drop.owner_id
                    )),
                });

                // Loop through each password and add it to the lookup map
                for pw in pws {
                    require!(
                        pw.key_use <= num_uses_per_key,
                        "claim out of range for password"
                    );
                    pw_map.insert(&pw.key_use, &hex::decode(pw.pw.clone()).unwrap());
                }

                pw_per_use = Some(pw_map);
            }

            require!(
                pw_per_use.is_some() as u8 + pw_per_key.is_some() as u8 <= 1,
                "You cannot specify both local and global passwords for a key"
            );

            exiting_key_map.insert(
                &pk,
                &KeyInfo {
                    remaining_uses: num_uses_per_key,
                    last_used: 0, // Set to 0 since this will make the key always claimable.
                    allowance: actual_allowance,
                    key_id: next_key_id,
                    pw_per_use,
                    pw_per_key,
                },
            );
            require!(
                self.drop_id_for_pk.insert(&pk, &drop_id.0).is_none(),
                "Keys cannot belong to another drop"
            );
            next_key_id += 1;
            idx += 1;
        }

        // Set the drop's PKs to the newly populated set
        drop.pks = exiting_key_map;
        // Set the drop's current key nonce
        drop.next_key_id = next_key_id;

        // Decide what methods the access keys can call
        let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
        if let Some(perms) = config.clone().and_then(|c| c.usage).and_then(|u| u.permissions) {
            match perms {
                // If we have a config, use the config to determine what methods the access keys can call
                ClaimPermissions::claim => {
                    access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
                }
                ClaimPermissions::create_account_and_claim => {
                    access_key_method_names = ACCESS_KEY_CREATE_ACCOUNT_METHOD_NAME;
                }
            }
        }

        // Should we register the keys for simple drops now or later?
        let mut lazy_register = false;

        // Increment the uses registered if drop is FC or Simple
        match &drop.drop_type {
            DropType::fc(data) => {
                drop.registered_uses += num_uses_per_key * len as u64;

                // If GAS is specified, set the GAS to attach for allowance calculations
                if let Some(_) = data.config.clone().and_then(|c| c.attached_gas) {
                    access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
                }
            }
            DropType::simple(simple_data) => {
                lazy_register = simple_data.lazy_register.unwrap_or(false);

                if !lazy_register {
                    drop.registered_uses += num_uses_per_key * len as u64;
                }
            }
            _ => {}
        };

        // Add the drop back in for the drop ID
        self.drop_for_id.insert(&drop_id.0, &drop);

        // Get the current balance of the funder.
        let mut current_user_balance = self.user_balances.get(&funder).unwrap_or(0);
        let near_attached = env::attached_deposit();
        // Add the attached deposit to their balance
        current_user_balance += near_attached;
        self.user_balances.insert(&funder, &current_user_balance);

        // Get the required attached_deposit for all the FCs
        let mut deposit_required_for_fc_deposits = 0;
        // Get the number of none FCs in FCData (if there are any)
        let mut num_none_fcs = 0;
        if let DropType::fc(data) = &drop.drop_type {
            // Ensure proper method data is passed in
            let num_method_data = data.clone().methods.len() as u64;

            // If there's one method data specified and more than 1 claim per key, that data is to be used
            // For all the uses. In this case, we need to tally all the deposits for each method in all method data.
            if num_uses_per_key > 1 && num_method_data == 1 {
                let attached_deposit = data
                    .methods
                    .iter()
                    .next()
                    .unwrap()
                    .clone()
                    .expect("cannot have a single none function call")
                    // iterate through   all entries and sum the attached_deposit
                    .iter()
                    .fold(0, |acc, x| acc + x.attached_deposit.0);

                deposit_required_for_fc_deposits = num_uses_per_key as u128 * attached_deposit;
            // In the case where either there's 1 claim per key or the number of FCs is not 1,
            // We can simply loop through and manually get this data
            } else {
                for method_name in data.methods.clone() {
                    num_none_fcs += method_name.is_none() as u64;
                    // If the method is not None, we need to get the attached_deposit by looping through the method datas
                    if let Some(method_data) = method_name {
                        let attached_deposit = method_data
                            .iter()
                            .fold(0, |acc, x| acc + x.attached_deposit.0);
                        deposit_required_for_fc_deposits += attached_deposit;
                    }
                }
            }
        }

        // Get optional costs
        let mut ft_optional_costs_per_claim = 0;
        match drop.drop_type {
            DropType::ft(data) => ft_optional_costs_per_claim = data.ft_storage.0,
            _ => {}
        };

        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_required_storage =
            Balance::from(final_storage - initial_storage) * env::storage_byte_cost();

        /*
            Required attached_deposit consists of:
            - Fees
            - TOTAL Storage
            - Total access key allowance for EACH key
            - Access key storage for EACH key
            - Balance for each key * (number of uses - uses with None for FC Data)

            Optional:
            - FC attached_deposit for each key * num Some(data) uses
            - FT storage registration cost for each key * uses (calculated in resolve storage calculation function)
        */
        let fees = self
            .fees_per_user
            .get(&funder)
            .unwrap_or((self.drop_fee, self.key_fee));
        let drop_fee = fees.0;
        let key_fee = fees.1;
        let total_key_fee = key_fee * len;
        let total_allowance = actual_allowance * len;
        let total_access_key_storage = ACCESS_KEY_STORAGE * len;
        let total_deposits = (drop.deposit_per_use * (num_uses_per_key - num_none_fcs) as u128 * len) * !lazy_register as u128;
        let total_deposits_for_fc = deposit_required_for_fc_deposits * len;
        let total_ft_costs = ft_optional_costs_per_claim * num_uses_per_key as u128 * len;

        let required_deposit = total_required_storage
            + total_key_fee
            + total_allowance
            + total_access_key_storage
            + total_deposits
            + total_ft_costs
            + total_deposits_for_fc;

        near_sdk::log!(
            "Current balance: {}, 
            Required Deposit: {}, 
            Total required storage: {},
            Drop Fee: {}, 
            Key Fee: {} Total Key Fee: {},
            allowance: {} total allowance: {},
            access key storage: {} total access key storage: {},
            deposits less none FCs: {} total deposits: {} lazy registration: {},
            deposits for FCs: {} total deposits for FCs: {},
            FT Costs per claim {} total FT Costs: {},
            uses per key: {}
            None FCs: {},
            length: {}",
            yocto_to_near(current_user_balance),
            yocto_to_near(required_deposit),
            yocto_to_near(total_required_storage),
            yocto_to_near(drop_fee),
            yocto_to_near(key_fee),
            yocto_to_near(total_key_fee),
            yocto_to_near(actual_allowance),
            yocto_to_near(total_allowance),
            yocto_to_near(ACCESS_KEY_STORAGE),
            yocto_to_near(total_access_key_storage),
            yocto_to_near(drop.deposit_per_use * (num_uses_per_key - num_none_fcs) as u128),
            yocto_to_near(total_deposits),
            lazy_register,
            yocto_to_near(deposit_required_for_fc_deposits),
            yocto_to_near(total_deposits_for_fc),
            yocto_to_near(ft_optional_costs_per_claim),
            yocto_to_near(total_ft_costs),
            num_uses_per_key,
            num_none_fcs,
            len
        );

        /*
            Ensure the attached attached_deposit can cover:
        */
        if current_user_balance < required_deposit {
            near_sdk::log!(
                "Not enough user balance. Found {} expected: {}",
                yocto_to_near(current_user_balance),
                yocto_to_near(required_deposit)
            );
            current_user_balance -= near_attached;

            // If they have a balance, insert it back into the map otherwise remove it
            if current_user_balance > 0 {
                self.user_balances.insert(&funder, &current_user_balance);
            } else {
                self.user_balances.remove(&funder);
            }

            // Refund the predecessor for their attached deposit if it's greater than 0
            if near_attached > 0 {
                Promise::new(env::predecessor_account_id()).transfer(near_attached);
            }

            // Remove the drop
            self.internal_remove_drop(&drop_id.0, keys_to_iter);
            // Return early
            return None;
        }
        require!(
            current_user_balance >= required_deposit,
            "Not enough attached_deposit"
        );
        // Decrement the user's balance by the required attached_deposit and insert back into the map
        current_user_balance -= required_deposit;
        self.user_balances.insert(&funder, &current_user_balance);
        near_sdk::log!("New user balance {}", yocto_to_near(current_user_balance));

        // Increment our fees earned
        self.fees_collected += fees.1 * len;
        near_sdk::log!("Fees collected {}", yocto_to_near(fees.1 * len));

        // Create a new promise batch to create all the access keys
        let current_account_id = env::current_account_id();
        let promise = env::promise_batch_create(&current_account_id);

        // Loop through each public key and create the access keys
        for pk in keys_to_iter.clone() {
            // Must assert in the loop so no access keys are made?
            env::promise_batch_action_add_key_with_function_call(
                promise,
                &pk,
                0,
                actual_allowance,
                &current_account_id,
                access_key_method_names,
            );
        }

        env::promise_return(promise);

        Some(drop_id)
    }

    #[payable]
    /// Allows a user to register uses for a simple lazy registration drop. They can over-register if they would like.
    pub fn register_uses(
        &mut self,
        drop_id: DropIdJson,
        num_uses: u64
    ) {
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        require!(env::predecessor_account_id() == drop.owner_id, "only owner can register uses");

        // Get the attached deposit and add it to the user's balance
        let attached_deposit = env::attached_deposit();
        if attached_deposit > 0 {
            self.internal_modify_user_balance(&env::predecessor_account_id(), attached_deposit, false);
        }

        // Ensure the drop's drop type is simple
        if let DropType::simple(simple_data) = &drop.drop_type {
            require!(simple_data.lazy_register == Some(true), "drop must be set to lazy register");

            // Get the required cost and decrement the user balance. If the user balance is not enough, the decrement will fail
            let required_cost = num_uses as u128 * drop.deposit_per_use;
            self.internal_modify_user_balance(&env::predecessor_account_id(), required_cost, true);

            // Increment the registered uses
            drop.registered_uses += num_uses;
            self.drop_for_id.insert(&drop_id.0, &drop);
        } else {
            env::panic_str("Drop must be simple type");
        }
    }

    #[payable]
    /// Allows a user to unregister uses for a simple lazy registration drop and get a refund to their user-balance.
    pub fn unregister_uses(
        &mut self,
        drop_id: DropIdJson,
        num_uses: u64
    ) {
        // Get the drop and ensure the owner is calling this method
        let mut drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        require!(env::predecessor_account_id() == drop.owner_id, "only owner can register uses");

        // Ensure the drop's drop type is simple
        if let DropType::simple(simple_data) = &drop.drop_type {
            require!(simple_data.lazy_register == Some(true), "drop must be set to lazy register");
            require!(num_uses <= drop.registered_uses, "cannot unregister more uses than registered");

            // The refund amount is however many uses to unregister times the deposit per use
            let refund_amount = num_uses as u128 * drop.deposit_per_use;
            self.internal_modify_user_balance(&env::predecessor_account_id(), refund_amount, false);

            // Decrement the registered uses
            drop.registered_uses -= num_uses;
            self.drop_for_id.insert(&drop_id.0, &drop);
        } else {
            env::panic_str("Drop must be simple type");
        }
    }
}
