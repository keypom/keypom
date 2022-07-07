use crate::*;
use near_sdk::GasWeight;

/// Keep track fungible token data for an access key. This is stored on the contract
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FTData {
    pub ft_contract: AccountId,
    pub ft_sender: AccountId,
    pub ft_balance: U128,
    pub ft_storage: U128,
}

/// FT Data to be passed in by the user
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FTDataConfig {
    pub ft_contract: AccountId,
    pub ft_sender: AccountId,
    pub ft_balance: U128,
}

// Returned from the storage balance bounds cross contract call on the FT contract
#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalanceBounds {
    pub min: U128,
    pub max: Option<U128>,
}

#[near_bindgen]
impl DropZone {
    /// Allows users to attach fungible tokens to the Linkdrops. Must have storage recorded by this point. You can only attach one set of FTs or NFT at a time.
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: U128,
    ) -> PromiseOrValue<U128> {
        let contract_id = env::predecessor_account_id();

        let mut drop = self.drop_for_id.get(&msg.0).expect("No drop found for ID");
        if let DropType::FT(ft_data) = &drop.drop_type {
            require!(amount.0 % ft_data.ft_balance.0 == 0, "amount must be a multiple of the drop balance");
            require!(ft_data.ft_contract == contract_id && ft_data.ft_sender == sender_id, "FT data must match what was sent");
            
            // Get the number of claims to register with the amount that is sent.
            let claims_to_register = (amount.0 / ft_data.ft_balance.0) as u64;
            drop.num_claims_registered += claims_to_register;
            env::log_str(&format!("New claims registered {}", claims_to_register));
    
            // Ensure that the keys to register can't exceed the number of keys in the drop.
            if drop.num_claims_registered > drop.pks.len() * drop.drop_config.max_claims_per_key {
                env::log_str("Too many FTs sent. Contract is keeping the rest.");
                drop.num_claims_registered = drop.pks.len() * drop.drop_config.max_claims_per_key;
            }
    
            // Insert the drop with the updated data
            self.drop_for_id.insert(&msg.0, &drop);

            // Everything went well and we don't need to return any tokens (if they over-sent, we keep it)
            PromiseOrValue::Value(U128(0))
        } else {
            env::panic_str("drop type isn't FT");
        }
    }

    #[private]
    /// Self callback checks if fungible tokens were successfully transferred to the new account. If yes, do nothing. If no, refund original sender
    pub fn ft_resolve_batch(
        &mut self, 
        amount: U128, 
        token_sender: AccountId,
        token_contract: AccountId
    ) -> bool {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of resolve transfer used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));
        
        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();
        env::log_str(&format!("Before refunding token sender in resolve transfer: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        if transfer_succeeded {
            return true
        }
        // Create a new batch promise to pay storage and refund the FTs to the original sender 
        let batch_ft_promise_id = env::promise_batch_create(&token_contract);

        // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
        // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
        env::promise_batch_action_function_call_weight(
            batch_ft_promise_id,
            "storage_deposit",
            json!({ "account_id": token_sender }).to_string().as_bytes(),
            amount.0,
            MIN_GAS_FOR_STORAGE_DEPOSIT,
            GasWeight(1)
        );

        // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
        // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
        env::promise_batch_action_function_call_weight(
            batch_ft_promise_id,
            "ft_transfer",
            json!({ "receiver_id": token_sender, "amount": amount, "memo": "Refunding Linkdropped FT Tokens" }).to_string().as_bytes(),
            1,
            MIN_GAS_FOR_FT_TRANSFER,
            GasWeight(1)
        );

        // Return the result of the batch as the return of the function
        env::promise_return(batch_ft_promise_id);

        false
    }

    #[private]
    /// Self callback checks if fungible tokens were successfully refunded. If yes, set keys registered to 0.
    pub fn ft_resolve_refund(
        &mut self, 
        drop_id: DropId,
        num_to_refund: u64
    ) -> bool {
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));
    
        // Everything went well so we return true since the keys registered have already been decremented
        if transfer_succeeded {
            env::log_str(&format!("Successfully refunded FTs for drop ID {}. {} keys unregistered. Returning true.", drop_id, num_to_refund));
            return true
        }

        // Transfer failed so we need to increment the claims registered and return false
        let mut drop = self.drop_for_id.get(&drop_id).expect("no drop for ID");
        drop.num_claims_registered += num_to_refund;
        self.drop_for_id.insert(&drop_id, &drop);

        env::log_str(&format!("Unsuccessful refund for drop ID {}. {} keys added back as registered. Returning false.", drop_id, num_to_refund));
        false
    }

    #[payable]
    #[private]
    /// self callback gets the storage balance bounds and inserts that into account data for each public key passed in
    pub fn resolve_storage_check(
        &mut self,
        public_keys: Vec<PublicKey>,
        drop_id: DropId,
        required_deposit: u128,
    ) -> bool {
        // Check promise result.
        let result = promise_result_as_success();
        let pub_keys_len = public_keys.len() as u128;

        // If things went wrong, we need to delete the data and refund the user.
        if result.is_none() {
            // Refund the funder any excess $NEAR
            env::log_str(&format!("Unsuccessful query to get storage. Refunding funder's balance: {}", yocto_to_near(required_deposit)));
            // Remove the drop
            let mut drop = self.drop_for_id.remove(&drop_id).expect("drop not found");
            // Clear the map
            drop.pks.clear();
            let funder_id = drop.funder_id.clone();
            
            // Remove the drop ID from the funder's list
            self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
            
            // Loop through the keys and remove the public keys' mapping
            for pk in public_keys {
                self.drop_id_for_pk.remove(&pk.clone());
            }
            
            // Refund the user's balance for the required deposit
            let mut user_balance = self.user_balances.get(&funder_id).unwrap();
            user_balance += required_deposit;
            self.user_balances.insert(&funder_id, &user_balance);

            return false;
        }

        // Try to get the storage balance bounds from the result of the promise
		if let Ok(StorageBalanceBounds{ min, max: _ }) = near_sdk::serde_json::from_slice::<StorageBalanceBounds>(&result.unwrap()) {
            let mut drop = self.drop_for_id.get(&drop_id).unwrap();
            let funder_id = drop.funder_id.clone();

            // Get the current user balance ad ensure that they have the extra $NEAR for covering the FT storage
            let mut cur_user_balance = self.user_balances.get(&funder_id).unwrap();
            let extra_storage_required =  min.0 * drop.drop_config.max_claims_per_key as u128 * pub_keys_len;
            
            // Ensure the user's current balance can cover the extra storage required
            if cur_user_balance < extra_storage_required {
                env::log_str(&format!("Not enough balance to cover FT storage for each key and their claims. Refunding funder's balance: {}", yocto_to_near(required_deposit)));
                // Remove the drop
                self.drop_for_id.remove(&drop_id).unwrap();
                // Clear the map
                drop.pks.clear();
                
                // Remove the drop ID from the funder's list
                self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
                
                // Loop through the keys and remove the keys from the drop and remove the drop ID for the key
                for pk in public_keys {
                    self.drop_id_for_pk.remove(&pk.clone());
                }

                // Refund the user's balance for the required deposit
                cur_user_balance += required_deposit;
                self.user_balances.insert(&funder_id, &cur_user_balance);

                return false;
            }

            // Update the FT data to include the storage and insert the drop back with the updated FT data
            if let DropType::FT(mut ft_data) = drop.drop_type {
                ft_data.ft_storage = min;
                drop.drop_type = DropType::FT(ft_data);
    
                self.drop_for_id.insert(
                    &drop_id, 
                    &drop
                );
    
                // Decrement the user's balance by the extra required and insert back into the map
                cur_user_balance -= extra_storage_required;
                self.user_balances.insert(&funder_id, &cur_user_balance);
    
                // Create the keys for the contract
                let promise = env::promise_batch_create(&env::current_account_id());
            
                // Decide what methods the access keys can call
                let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
                if drop.drop_config.only_call_claim.is_some() {
                    access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
                }

                // Dynamically calculate the access key allowance based on the base + number of claims per key * 100 TGas
                let access_key_allowance = BASE_ACCESS_KEY_ALLOWANCE + (drop.drop_config.max_claims_per_key - 1) as u128 * ATTACHED_GAS_FROM_WALLET.0 as u128 * self.yocto_per_gas;

                // Loop through each public key and create the access keys
                for pk in public_keys.clone() {
                    env::promise_batch_action_add_key_with_function_call(
                        promise, 
                        &pk, 
                        0, 
                        access_key_allowance, 
                        &env::current_account_id(), 
                        access_key_method_names
                    );
                }
    
                env::promise_return(promise);
    
                // Everything went well and we return true
                return true
            } else {
                false
            }

        } else {
            // Refund the funder any excess $NEAR
            env::log_str(&format!("Unsuccessful query to get storage. Refunding funder's balance: {}", yocto_to_near(required_deposit)));
            // Remove the drop
            let mut drop = self.drop_for_id.remove(&drop_id).expect("drop not found");
            // Clear the map
            drop.pks.clear();
            let funder_id = drop.funder_id.clone();
            
            // Remove the drop ID from the funder's list
            self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
            
            // Loop through the keys and remove the public keys' mapping
            for pk in public_keys {
                self.drop_id_for_pk.remove(&pk.clone());
            }
            
            // Refund the user's balance for the required deposit
            let mut user_balance = self.user_balances.get(&funder_id).unwrap();
            user_balance += required_deposit;
            self.user_balances.insert(&funder_id, &user_balance);

            return false;
        }
    }

    // Internal method for transfer FTs. Whether the claim was successful or not is passed in
    pub(crate) fn internal_ft_transfer(
        &mut self,
        claim_succeeded: bool,
        ft_contract: AccountId,
        ft_balance: U128,
        ft_storage: U128,
        ft_sender: AccountId,
        account_id: AccountId
    ) {
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
    }
}
