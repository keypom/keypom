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
impl LinkDropProxy {
    /// Allows users to attach fungible tokens to the Linkdrops. Must have storage recorded by this point. You can only attach one set of FTs or NFT at a time.
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: PublicKey,
    ) -> PromiseOrValue<U128> {
        let contract_id = env::predecessor_account_id();

        // No need to assert that the funder is the sender since we don't wanna enforce anything unnecessary.
        // All that matters is we've received the FTs and that they belongs to some public key.
        let AccountData {
            funder_id,
            balance,
            storage_used,
            cb_id,
            cb_data_sent,
        } = self.accounts
            .get(&msg)
            .expect("Missing public key");

        // Ensure there's a callback ID (meaning the linkdrop is not a regular linkdrop)
        let callback_id = cb_id.expect("Callback ID must be set");

        // Assert that the FTs have NOT been sent yet
        assert!(cb_data_sent == false, "FTs already sent. Cannot send more.");

        // Ensure that the linkdrop contains FT data already
        let FTData { 
            ft_contract,
            ft_sender, 
            ft_balance, 
            ft_storage: _ 
        } = self.ft.get(&callback_id).expect("No FT data found for the unique callback ID.");

        assert!(ft_contract == contract_id && ft_sender == sender_id && ft_balance == amount, "FT data must match what was sent");
        
        // Insert the account data back with the cb data sent set to true
        self.accounts.insert(
            &msg,
            &AccountData{
                funder_id,
                balance,
                storage_used,
                cb_id,
                cb_data_sent: true,
            },
        );

        // Everything went well and we don't need to return any tokens
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
        env::promise_batch_action_function_call(
            batch_ft_promise_id,
            "storage_deposit",
            json!({ "account_id": token_sender }).to_string().as_bytes(),
            amount.0,
            GAS_FOR_STORAGE_DEPOSIT
        );

        // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
        env::promise_batch_action_function_call(
            batch_ft_promise_id,
            "ft_transfer",
            json!({ "receiver_id": token_sender, "amount": amount, "memo": "Refunding Linkdropped FT Tokens" }).to_string().as_bytes(),
            1,
            GAS_FOR_FT_TRANSFER
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
        funder_id: AccountId,
        balance: U128,
        required_storage: U128,
        cb_ids: Vec<u64>,
    ) -> bool {
        let attached_deposit = env::attached_deposit();
        let len = public_keys.len() as u128;

        // Check promise result.
        let result = promise_result_as_success();

        if result.is_none() || cb_ids.len() as u128 != len {
            // Refund the funder any excess $NEAR and panic which will cause generic $NEAR linkdrops to be used
            env::log_str("Unsuccessful query to get storage. Refunding funder excess $NEAR and generic $NEAR linkdrop will be used.");
            Promise::new(funder_id.clone()).transfer(attached_deposit - (ACCESS_KEY_STORAGE + required_storage.0 + ACCESS_KEY_ALLOWANCE + balance.0) * len);
            for cb_id in cb_ids {
                self.ft.remove(&cb_id);
            }
            return false;
        }

        // Try to get the storage balance bounds from the result of the promise
		if let Ok(StorageBalanceBounds{ min, max: _ }) = near_sdk::serde_json::from_slice::<StorageBalanceBounds>(&result.unwrap()) {
            // Ensure the user attached enough to cover the regular $NEAR linkdrops case PLUS the storage for the fungible token contract for each key
            
            if attached_deposit < attached_deposit - (ACCESS_KEY_STORAGE + required_storage.0 + ACCESS_KEY_ALLOWANCE + balance.0 + min.0) * len {
                env::log_str("Deposit must be large enough to cover desired balance, access key allowance, and contract storage");
                for cb_id in cb_ids {
                    self.ft.remove(&cb_id);
                }
                return false;
            }

            let mut index = 0;
            // Loop through each public key and insert them into the map with the new FT storage
            for _pk_ in public_keys {
                // Get current FT data excluding the storage
                let FTData { 
                    ft_contract,
                    ft_sender, 
                    ft_balance, 
                    ft_storage: _
                } = self.ft.get(&cb_ids[index]).expect("No FT data found for the unique callback ID.");

                // Insert the FT data including the new storage for the unique callback ID associated with the linkdrop
                self.ft.insert(
                    &cb_ids[index], 
                    &FTData { 
                        ft_contract: ft_contract,
                        ft_sender: ft_sender, 
                        ft_balance: ft_balance, 
                        ft_storage: Some(min),
                    }
                );
                index += 1;
            }

            // If the user overpaid for the desired linkdrop balance, refund them.
            if attached_deposit > (ACCESS_KEY_STORAGE + required_storage.0 + ACCESS_KEY_ALLOWANCE + balance.0 + min.0) * len {    
                Promise::new(funder_id).transfer(attached_deposit - (ACCESS_KEY_STORAGE + required_storage.0 + ACCESS_KEY_ALLOWANCE + balance.0 + min.0) * len);
            }
            
            // Everything went well and we return true
            return true
        } else {
            env::log_str("Unsuccessful query to get storage. Refunding funder excess $NEAR and generic $NEAR linkdrop will be used.");
            // Refund the funder any excess $NEAR and panic which will cause generic $NEAR linkdrops to be used
            Promise::new(funder_id.clone()).transfer(attached_deposit - (ACCESS_KEY_STORAGE + required_storage.0 + ACCESS_KEY_ALLOWANCE + balance.0) * len);
            for cb_id in cb_ids {
                self.ft.remove(&cb_id);
            }
            return false;
        }
    }
}
