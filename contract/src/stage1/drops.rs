use near_sdk::{Balance, require};

use crate::{*, views::JsonNFTData};

#[near_bindgen]
impl DropZone {
    /*
        user has created a bunch of keypairs and passed in the public keys and attached some deposit.
        this will store the account data and allow that keys to call claim and create_account_and_claim
        on this contract.

        The balance is the amount of $NEAR the sender wants each linkdrop to contain.
    */
    #[payable]
    pub fn create_drop(
        &mut self, 
        public_keys: Vec<PublicKey>, 
        balance: U128,
        ft_data: Option<FTData>,
        nft_data: Option<JsonNFTData>,
        fc_data: Option<FCData>
    ) -> DropId {
        // Ensure the user has only specified one type of callback data
        let num_cbs_specified = if ft_data.is_some() {1} else {0} + if nft_data.is_some() {1} else {0} + if fc_data.is_some() {1} else {0};
        env::log_str(&format!("Num CBs {}", num_cbs_specified));
        require!(num_cbs_specified <= 1, "You cannot specify more than one callback data");

        // Ensure the balance for each drop is larger than the minimum
        require!(
            balance.0 >= NEW_ACCOUNT_BASE,
            "cannot have a desired account balance less than the absolute minimum for creating an account"
        );

        // Funder is the predecessor
        let funder_id = env::predecessor_account_id();
        let len = public_keys.len() as u128;
        let drop_id = self.nonce;

        // Get the current balance of the funder. 
        let mut current_user_balance = self.user_balances.get(&funder_id).expect("No user balance found");
        env::log_str(&format!("Cur User balance {}", yocto_to_near(current_user_balance)));
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();

        let mut key_set: UnorderedSet<PublicKey> = UnorderedSet::new(StorageKey::DropIdsForFunderInner {
            //we get a new unique prefix for the collection
            account_id_hash: hash_account_id(&format!("{}{}", self.nonce, funder_id)),
        });
        
        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in public_keys.clone() {
            key_set.insert(&pk);
            require!(self.drop_id_for_pk.insert(&pk, &drop_id).is_none(), "Keys cannot belong to another drop");
        }

        // Create drop object 
        let mut drop = Drop { 
            funder_id: env::predecessor_account_id(), 
            balance, 
            pks: key_set,
            ft_data: ft_data.clone(),
            nft_data: None, // Defaulting to None
            fc_data: fc_data.clone(),
            storage_used_per_key: U128(u128::max_value()),
            keys_registered: 0
        };

        // For NFT drops, measure the storage for adding the longest token ID
        let mut storage_per_longest = 0;

        // If NFT data was provided, we need to build the set of token IDs
        if nft_data.is_some() {
            let JsonNFTData{nft_sender, nft_contract, longest_token_id} = nft_data.unwrap();

            // Create the token ID set and insert the longest token ID
            let mut token_ids = UnorderedSet::new(StorageKey::TokenIdsForDrop {
                //we get a new unique prefix for the collection
                account_id_hash: hash_account_id(&format!("nft-{}{}", self.nonce, funder_id)),
            });
            token_ids.insert(&longest_token_id);

            // Create the NFT data
            let actual_nft_data = NFTData {
                nft_sender,
                nft_contract,
                longest_token_id: longest_token_id,
                storage_for_longest: u128::MAX,
                token_ids: Some(token_ids)
            };
            
            // Measure how much storage it costs to insert the 1 longest token ID
            let initial_nft_storage = env::storage_usage();
            drop.nft_data = Some(actual_nft_data);

            // Add drop with the longest possible token ID and max storage
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );

            let final_nft_storage = env::storage_usage();
            // Measure the storage per single longest token ID
            storage_per_longest = Balance::from(final_nft_storage - initial_nft_storage);

            env::log_str(&format!("Storage per longest {}", storage_per_longest));
            // Clear the token IDs so it's an empty set and put the storage in the drop's nft data
            if let Some(mut data) = drop.nft_data {
                if let Some(mut ids) = data.token_ids {
                    ids.clear();
                    data.token_ids = Some(ids);
                }

                data.storage_for_longest = storage_per_longest;
                drop.nft_data = Some(data);
            }
        }

        // Add this drop ID to the funder's set of drops
        self.internal_add_drop_to_funder(&env::predecessor_account_id(), &drop_id);

        // Add drop with largest possible storage used and keys registered for now.
        self.drop_for_id.insert(
            &drop_id, 
            &drop
        );

        // TODO: add storage for access keys * num of public keys
        // Calculate the storage being used for the entire drop and add it to the drop.
        let final_storage = env::storage_usage();
        let total_required_storage = (Balance::from(final_storage - initial_storage) + storage_per_longest) * env::storage_byte_cost();
        env::log_str(&format!("Total required storage {}", yocto_to_near(total_required_storage)));

        // Insert the drop back with the storage
        drop.storage_used_per_key = U128(total_required_storage / len);

        // Overwrite the drop with the correct info now that storage has been measured
        self.drop_for_id.insert(
            &drop_id, 
            &drop
        );

        // Increment the drop ID nonce
        self.nonce += 1;

        let required_deposit = self.drop_fee + total_required_storage + (self.key_fee + ACCESS_KEY_STORAGE + ACCESS_KEY_ALLOWANCE + balance.0 + if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0}) * len;
        env::log_str(&format!(
            "Current balance: {}, 
            Required Deposit: {}, 
            Drop Fee: {}, 
            Key Fee: {}, 
            Total Required Storage: {}, 
            ACCESS_KEY_STORAGE: {},
            ACCESS_KEY_ALLOWANCE: {}, 
            Linkdrop Balance: {}, 
            total function call deposits (if applicable): {}, 
            length: {}", 
            yocto_to_near(current_user_balance), 
            yocto_to_near(required_deposit),
            yocto_to_near(self.drop_fee),
            yocto_to_near(self.key_fee),
            yocto_to_near(total_required_storage), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(ACCESS_KEY_ALLOWANCE), 
            yocto_to_near(balance.0), 
            yocto_to_near(if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0}), 
            len)
        );
        /*
            Ensure the attached deposit can cover: 
        */ 
        require!(current_user_balance >= required_deposit, "Not enough deposit");
        // Decrement the user's balance by the required deposit and insert back into the map
        current_user_balance -= required_deposit;
        self.user_balances.insert(&funder_id, &current_user_balance);
        env::log_str(&format!("New user balance {}", yocto_to_near(current_user_balance)));

        // Increment our fees earned
        self.fees_collected += self.drop_fee + self.key_fee * len;
        env::log_str(&format!("Fees collected {}", yocto_to_near(self.drop_fee + self.key_fee * len)));

        let current_account_id = env::current_account_id();
        
        /*
            Only add the access keys if it's not a FT drop. If it is,
            keys will be added in the FT resolver
        */
        if ft_data.is_none() {
            // Create a new promise batch to create all the access keys
            let promise = env::promise_batch_create(&current_account_id);
            
            // Loop through each public key and create the access keys
            for pk in public_keys.clone() {
                // Must assert in the loop so no access keys are made?
                env::promise_batch_action_add_key_with_function_call(
                    promise, 
                    &pk, 
                    0, 
                    ACCESS_KEY_ALLOWANCE, 
                    &current_account_id, 
                    ACCESS_KEY_METHOD_NAMES
                );
            }

            env::promise_return(promise);
        } else {
            /*
                Get the storage required by the FT contract and ensure the user has attached enough
                deposit to cover the storage and perform refunds if they overpayed.
            */ 

            ext_ft_contract::ext(ft_data.unwrap().ft_contract)
                // Call storage balance bounds with exactly this amount of GAS. No unspent GAS will be added on top.
                .with_static_gas(GAS_FOR_STORAGE_BALANCE_BOUNDS)
                .with_unused_gas_weight(0)
                .storage_balance_bounds()
            .then(
                Self::ext(current_account_id)
                    // Resolve the promise with the min GAS. All unspent GAS will be added to this call.
                    .with_static_gas(MIN_GAS_FOR_RESOLVE_STORAGE_CHECK)
                    .resolve_storage_check(
                        public_keys,
                        drop_id,
                        required_deposit
                    )
            );
        }

        drop_id
    }

    /*
        Allows users to add to an existing drop.
        Only the funder can call this method
    */
    #[payable]
    pub fn add_to_drop(
        &mut self, 
        public_keys: Vec<PublicKey>, 
        drop_id: DropId
    ) -> DropId {
        let mut drop = self.drop_for_id.get(&drop_id).expect("no drop found for ID");
        let funder = &drop.funder_id;

        require!(funder == &env::predecessor_account_id(), "only funder can add to drops");

        /*
            Add data to storage
        */
        // get the existing key set and add new PKs
        let mut exiting_key_set = drop.pks;
        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in public_keys.clone() {
            exiting_key_set.insert(&pk);
            require!(self.drop_id_for_pk.insert(&pk, &drop_id).is_none(), "Keys cannot belong to another drop");
        }

        // Set the drop's PKs to the newly populated set
        drop.pks = exiting_key_set;

        // Add the drop back in for the drop ID 
        self.drop_for_id.insert(
            &drop_id, 
            &drop
        );

        let len = public_keys.len() as u128;

        let fc_cost = if let Some(data) = drop.fc_data.clone() {
            data.deposit.0
        } else {
            0
        };

        let ft_cost = if let Some(data) = drop.ft_data.clone() {
            data.ft_storage.unwrap().0
        } else {
            0
        };
    
        let nft_cost = if let Some(data) = drop.nft_data {
            data.storage_for_longest * env::storage_byte_cost()
        } else {
            0
        };
        
        // Get the current balance of the funder. 
        let mut current_user_balance = self.user_balances.get(&funder).expect("No user balance found");
        env::log_str(&format!("Cur user balance {}", yocto_to_near(current_user_balance)));

        // Required deposit is the existing storage per key + key fee * length of public keys (plus all other basic stuff)
        let required_deposit = (drop.storage_used_per_key.0 + self.key_fee + ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + drop.balance.0 + fc_cost + ft_cost + nft_cost) * len;
        env::log_str(&format!(
            "Current User Balance: {}, 
            Required Deposit: {}, 
            Storage per key: {}, 
            Key fee: {}, 
            ACCESS_KEY_ALLOWANCE: {},
            ACCESS_KEY_STORAGE: {}, 
            Balance per key: {}, 
            function call costs: {}, 
            FT costs: {}, 
            NFT costs: {}, 
            length: {}", 
            yocto_to_near(current_user_balance), 
            yocto_to_near(required_deposit),
            yocto_to_near(drop.storage_used_per_key.0),
            yocto_to_near(self.key_fee), 
            yocto_to_near(ACCESS_KEY_ALLOWANCE), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(drop.balance.0), 
            yocto_to_near(fc_cost), 
            yocto_to_near(ft_cost), 
            yocto_to_near(nft_cost), 
            len)
        );
        /*
            Ensure the attached deposit can cover: 
        */ 
        require!(current_user_balance >= required_deposit, "Not enough deposit");
        // Decrement the user's balance by the required deposit and insert back into the map
        current_user_balance -= required_deposit;
        self.user_balances.insert(&funder, &current_user_balance);
        env::log_str(&format!("New user balance {}", yocto_to_near(current_user_balance)));

        // Increment our fees earned
        self.fees_collected += self.key_fee * len;
        env::log_str(&format!("Fees collected {}", yocto_to_near(self.key_fee * len)));
        
        // Create a new promise batch to create all the access keys
        let current_account_id = env::current_account_id();
        let promise = env::promise_batch_create(&current_account_id);
        
        // Loop through each public key and create the access keys
        for pk in public_keys.clone() {
            // Must assert in the loop so no access keys are made?
            env::promise_batch_action_add_key_with_function_call(
                promise, 
                &pk, 
                0, 
                ACCESS_KEY_ALLOWANCE, 
                &current_account_id, 
                ACCESS_KEY_METHOD_NAMES
            );
        }

        env::promise_return(promise);

        drop_id
    }
}