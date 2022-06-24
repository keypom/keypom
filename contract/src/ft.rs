use near_sdk::GasWeight;

use crate::*;


/// Keep track fungible token data for an access key
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FTData {
    pub ft_contract: AccountId,
    pub ft_sender: AccountId,
    pub ft_balance: U128,
    pub ft_storage: Option<U128>,
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
        msg: DropId,
    ) -> PromiseOrValue<U128> {
        let contract_id = env::predecessor_account_id();

        let mut drop = self.drop_type_for_id.get(&msg).expect("No drop found for ID");
        let FTData { ft_contract, ft_sender, ft_balance, ft_storage: _ } = drop.ft_data.as_ref().expect("No FT data found for drop");

        require!(ft_contract == &contract_id && ft_sender == &sender_id && amount.0 >= ft_balance.0, "FT data must match what was sent");
        
        // Get the number of keys to register with the amount that is sent.
        let keys_to_register = (amount.0 / ft_balance.0) as u64;
        drop.keys_registered += keys_to_register;

        // Ensure that the keys to register can't exceed the number of keys in the drop
        if drop.keys_registered > drop.pks.len() {
            drop.keys_registered = drop.pks.len()
        }

        // Insert the drop with the updated data
        self.drop_type_for_id.insert(&msg, &drop);

        // Everything went well and we don't need to return any tokens (if they over-sent, we keep it)
        PromiseOrValue::Value(U128(0))
    }

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

        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
        assert_eq!(env::promise_results_count(), 1, "no promise result");
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

    #[payable]
    /// self callback gets the storage balance bounds and inserts that into account data for each public key passed in
    pub fn resolve_storage_check(
        &mut self,
        public_keys: Vec<PublicKey>,
        drop_id: DropId
    ) -> bool {
        // Check promise result.
        let result = promise_result_as_success();
        let attached_deposit = env::attached_deposit();
        let pub_keys_len = public_keys.len() as u128;

        // If things went wrong, we need to delete the data and refund the user.
        if result.is_none() {
            // Refund the funder any excess $NEAR
            env::log_str("Unsuccessful query to get storage. Refunding funder.");
            // Remove the drop
            let mut drop = self.drop_type_for_id.remove(&drop_id).expect("drop type not found");
            let funder_id = drop.funder_id.clone();
            
            // Remove the drop ID from the funder's list
            self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
            
            // Loop through the keys and remove the keys from the drop and remove the drop ID for the key
            for pk in public_keys {
                self.drop_id_for_pk.remove(&pk.clone());

                // Remove the pk from the drop's set.
                drop.pks.remove(&pk);
            }
            
            // If there are keys still left in the drop, add the drop back in with updated data
            if !drop.pks.is_empty() {
                // Add drop type back with the updated data.
                self.drop_type_for_id.insert(
                    &drop_id, 
                    &drop
                );
            }

            // Refund the funder for their attached deposit
            Promise::new(funder_id).transfer(attached_deposit);

            return false;
        }

        // Try to get the storage balance bounds from the result of the promise
		if let Ok(StorageBalanceBounds{ min, max: _ }) = near_sdk::serde_json::from_slice::<StorageBalanceBounds>(&result.unwrap()) {
            let mut drop = self.drop_type_for_id.get(&drop_id).unwrap();
            
            // Ensure the user attached enough to cover the regular $NEAR linkdrops case PLUS the storage for the fungible token contract for each key
            let required_deposit =  (drop.storage_used_per_key.0 + ACCESS_KEY_ALLOWANCE + drop.balance.0 + min.0) * pub_keys_len;
            
            if attached_deposit < required_deposit {
                env::log_str("Deposit not large enough to cover FT storage for each key. Refunding funder.");
                // Remove the drop
                let mut drop = self.drop_type_for_id.remove(&drop_id).expect("drop type not found");
                let funder_id = drop.funder_id.clone();
                
                // Remove the drop ID from the funder's list
                self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
                
                // Loop through the keys and remove the keys from the drop and remove the drop ID for the key
                for pk in public_keys {
                    self.drop_id_for_pk.remove(&pk.clone());

                    // Remove the pk from the drop's set.
                    drop.pks.remove(&pk);
                }
                
                // If there are keys still left in the drop, add the drop back in with updated data
                if !drop.pks.is_empty() {
                    // Add drop type back with the updated data.
                    self.drop_type_for_id.insert(
                        &drop_id, 
                        &drop
                    );
                }

                // Refund the funder for their attached deposit
                Promise::new(funder_id).transfer(attached_deposit);

                return false;
            }

            // Update the FT data to include the storage and insert the drop back with the updated FT data
            let mut new_ft_data = drop.ft_data.clone().unwrap();
            new_ft_data.ft_storage = Some(min);
            drop.ft_data = Some(new_ft_data);

            self.drop_type_for_id.insert(
                &drop_id, 
                &drop
            );

            // If the user overpaid for the desired linkdrop balance, refund them.
            if attached_deposit > required_deposit {    
                env::log_str(&format!("Refunding User for: {}", yocto_to_near(attached_deposit - required_deposit)));
                Promise::new(drop.funder_id).transfer(attached_deposit - required_deposit);
            }
            
            // Everything went well and we return true
            return true
        } else {
            // Refund the funder any excess $NEAR
            env::log_str("Unsuccessful query to get storage. Refunding funder.");
            // Remove the drop
            let mut drop = self.drop_type_for_id.remove(&drop_id).expect("drop type not found");
            let funder_id = drop.funder_id.clone();
            
            // Remove the drop ID from the funder's list
            self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
            
            // Loop through the keys and remove the keys from the drop and remove the drop ID for the key
            for pk in public_keys {
                self.drop_id_for_pk.remove(&pk.clone());

                // Remove the pk from the drop's set.
                drop.pks.remove(&pk);
            }
            
            // If there are keys still left in the drop, add the drop back in with updated data
            if !drop.pks.is_empty() {
                // Add drop type back with the updated data.
                self.drop_type_for_id.insert(
                    &drop_id, 
                    &drop
                );
            }

            // Refund the funder for their attached deposit
            Promise::new(funder_id).transfer(attached_deposit);

            return false;
        }
    }
}
