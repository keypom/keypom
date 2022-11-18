use crate::*;
use near_sdk::GasWeight;

// Returned from the storage balance bounds cross contract call on the FT contract
#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalanceBounds {
    pub min: U128,
    pub max: Option<U128>,
}

#[near_bindgen]
impl Keypom {
    /// Allows users to attach fungible tokens to the Linkdrops. Must have storage recorded by this point. You can only attach one set of FTs or NFT at a time.
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: DropIdJson,
    ) -> PromiseOrValue<U128> {
        let contract_id = env::predecessor_account_id();

        let mut drop = self.drop_for_id.get(&msg.0).expect("No drop found for ID");
        if let DropType::ft(ft_data) = &drop.drop_type {
            require!(
                amount.0 % ft_data.balance_per_use.0 == 0,
                "amount must be a multiple of the drop balance"
            );
            require!(
                ft_data.contract_id == contract_id && ft_data.sender_id == sender_id,
                "FT data must match what was sent"
            );

            // Get the number of uses to register with the amount that is sent.
            let uses_to_register = (amount.0 / ft_data.balance_per_use.0) as u64;
            drop.registered_uses += uses_to_register;
            near_sdk::log!("New uses registered {}", uses_to_register);

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
        token_contract: AccountId,
    ) -> bool {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "Beginning of resolve transfer used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();
        near_sdk::log!(
            "Before refunding token sender in resolve transfer: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );

        if transfer_succeeded {
            return true;
        }
        // Create a new batch promise to pay storage and refund the FTs to the original sender
        let batch_ft_promise_id = env::promise_batch_create(&token_contract);

        // Send the fungible tokens (after the storage attached_deposit is finished since these run sequentially)
        // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
        env::promise_batch_action_function_call_weight(
            batch_ft_promise_id,
            "storage_deposit",
            json!({ "account_id": token_sender }).to_string().as_bytes(),
            amount.0,
            MIN_GAS_FOR_STORAGE_DEPOSIT,
            GasWeight(1),
        );

        // Send the fungible tokens (after the storage attached_deposit is finished since these run sequentially)
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
    pub fn ft_resolve_refund(&mut self, drop_id: DropId, num_to_refund: u64) -> bool {
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        // Everything went well so we return true since the keys registered have already been decremented
        if transfer_succeeded {
            near_sdk::log!(
                "Successfully refunded FTs for drop ID {}. {} keys unregistered.",
                drop_id,
                num_to_refund
            );
            return true;
        }

        // Transfer failed so we need to increment the uses registered and return false
        let mut drop = self.drop_for_id.get(&drop_id).expect("no drop for ID");
        drop.registered_uses += num_to_refund;
        self.drop_for_id.insert(&drop_id, &drop);

        near_sdk::log!("Unsuccessful refund for drop ID {}. {} keys added back as registered. Returning false.", drop_id, num_to_refund);
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

        // Get the attached deposit originally sent by the user (if any)
        let near_attached = env::attached_deposit();

        // If things went wrong, we need to delete the data and refund the user.
        if result.is_none() {
            // Remove the drop and get the owner ID back
            let owner_id = self.internal_remove_drop(&drop_id, public_keys);

            // Refund the user's balance for the required attached_deposit
            let mut user_balance = self.user_balances.get(&owner_id).unwrap();
            user_balance = user_balance + required_deposit - near_attached;

            // Refund the funder any excess $NEAR
            near_sdk::log!(
                "Unsuccessful query to get storage. Adding back req deposit of {} and subtracting near attached of {}. User balance is now {}",
                yocto_to_near(required_deposit),
                yocto_to_near(near_attached),
                yocto_to_near(user_balance)
            );

            if user_balance > 0 {
                // Refund the funder any excess $NEAR
                near_sdk::log!("User balance positive. Adding back into contract.");
                self.user_balances.insert(&owner_id, &user_balance);
            } else {
                near_sdk::log!("User balance zero. Removing from contract.");
                self.user_balances.remove(&owner_id);
            }

            // Refund the predecessor for their attached deposit if it's greater than 0
            if near_attached > 0 {
                near_sdk::log!(
                    "Refunding user for attached deposit of: {}",
                    yocto_to_near(near_attached)
                );

                Promise::new(env::predecessor_account_id()).transfer(near_attached);
            }

            return false;
        }

        // Try to get the storage balance bounds from the result of the promise
        if let Ok(StorageBalanceBounds { min, max: _ }) =
            near_sdk::serde_json::from_slice::<StorageBalanceBounds>(&result.unwrap())
        {
            let mut drop = self.drop_for_id.get(&drop_id).unwrap();
            let owner_id = drop.owner_id.clone();

            // Get the max uses per key. Default to 1 if not specified in the drop config.
            let uses_per_key = drop
                .config
                .clone()
                .and_then(|c| c.uses_per_key)
                .unwrap_or(1);

            // Get the current user balance ad ensure that they have the extra $NEAR for covering the FT storage
            let mut cur_user_balance = self.user_balances.get(&owner_id).unwrap();
            let extra_storage_required = min.0 * uses_per_key as u128 * pub_keys_len + min.0;

            // Ensure the user's current balance can cover the extra storage required
            if cur_user_balance < extra_storage_required {
                // Remove the drop and get the owner ID back
                let owner_id = self.internal_remove_drop(&drop_id, public_keys);

                // Refund the user's balance for the required attached_deposit
                let mut user_balance = self.user_balances.get(&owner_id).unwrap();
                user_balance = user_balance + required_deposit - near_attached;

                // Refund the funder any excess $NEAR
                near_sdk::log!(
                    "Not enough balance to cover FT storage for each key and their uses. Adding back req deposit of {} and subtracting near attached of {}. User balance is now {}",
                    yocto_to_near(required_deposit),
                    yocto_to_near(near_attached),
                    yocto_to_near(user_balance)
                );

                if user_balance > 0 {
                    // Refund the funder any excess $NEAR
                    near_sdk::log!("User balance positive. Adding back into contract.");
                    self.user_balances.insert(&owner_id, &user_balance);
                } else {
                    near_sdk::log!("User balance zero. Removing from contract.");
                    self.user_balances.remove(&owner_id);
                }

                // Refund the predecessor for their attached deposit if it's greater than 0
                if near_attached > 0 {
                    near_sdk::log!(
                        "Refunding user for attached deposit of: {}",
                        yocto_to_near(near_attached)
                    );

                    Promise::new(owner_id).transfer(near_attached);
                }

                return false;
            }

            // Update the FT data to include the storage and insert the drop back with the updated FT data
            if let DropType::ft(mut ft_data) = drop.drop_type {
                ft_data.ft_storage = min;
                drop.drop_type = DropType::ft(ft_data.clone());

                self.drop_for_id.insert(&drop_id, &drop);

                // Decrement the user's balance by the extra required and insert back into the map
                cur_user_balance -= extra_storage_required;
                near_sdk::log!(
                    "User has enough balance to cover FT storage. Subtracting {} from user balance. User balance is now {}",
                    yocto_to_near(extra_storage_required),
                    yocto_to_near(cur_user_balance)
                );
                self.user_balances.insert(&owner_id, &cur_user_balance);

                // Create the keys for the contract
                let promise = env::promise_batch_create(&env::current_account_id());

                // Decide what methods the access keys can call
                let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
                if let Some(perms) = drop.config.clone().and_then(|c| c.usage).and_then(|u| u.permissions) {
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

                // Get the number of uses per key
                let num_uses_per_key = drop.config.and_then(|c| c.uses_per_key).unwrap_or(1);
                // Calculate the base allowance to attach
                let calculated_base_allowance = self.calculate_base_allowance(drop.required_gas);
                // The actual allowance is the base * number of uses per key since each claim can potentially use the max pessimistic GAS.
                let actual_allowance = calculated_base_allowance * num_uses_per_key as u128;

                // Loop through each public key and create the access keys
                for pk in public_keys.clone() {
                    env::promise_batch_action_add_key_with_function_call(
                        promise,
                        &pk,
                        0,
                        actual_allowance,
                        &env::current_account_id(),
                        access_key_method_names,
                    );
                }

                env::promise_return(promise);

                self.internal_register_ft_contract(&ft_data.contract_id, min.0, &owner_id, true);

                // Everything went well and we return true
                return true;
            } else {
                false
            }
        } else {
            // Remove the drop and get the owner ID back
            let owner_id = self.internal_remove_drop(&drop_id, public_keys);

            // Refund the user's balance for the required attached_deposit
            let mut user_balance = self.user_balances.get(&owner_id).unwrap();
            user_balance = user_balance + required_deposit - near_attached;

            // Refund the funder any excess $NEAR
            near_sdk::log!(
                "Unsuccessful query to get storage. Adding back req deposit of {} and subtracting near attached of {}. User balance is now {}",
                yocto_to_near(required_deposit),
                yocto_to_near(near_attached),
                yocto_to_near(user_balance)
            );

            if user_balance > 0 {
                // Refund the funder any excess $NEAR
                near_sdk::log!("User balance positive. Adding back into contract.");
                self.user_balances.insert(&owner_id, &user_balance);
            } else {
                near_sdk::log!("User balance zero. Removing from contract.");
                self.user_balances.remove(&owner_id);
            }

            // Refund the predecessor for their attached deposit if it's greater than 0
            if near_attached > 0 {
                near_sdk::log!(
                    "Refunding user for attached deposit of: {}",
                    yocto_to_near(near_attached)
                );

                Promise::new(env::predecessor_account_id()).transfer(near_attached);
            }

            return false;
        }
    }

    // Internal method_name for transfer FTs. Whether the claim was successful or not is passed in
    pub(crate) fn internal_ft_transfer(
        &mut self,
        claim_succeeded: bool,
        ft_data: FTData,
        account_id: AccountId,
    ) {
        /*
            Fungible Tokens.
            - Only send the FTs if the sender ended up sending the contract the tokens.
        */
        // Only send the fungible tokens to the new account if the claim was successful. We return the FTs if it wasn't successful in the else case.
        if claim_succeeded {
            // Create a new batch promise to pay storage and transfer FTs to the new account ID
            let batch_ft_promise_id = env::promise_batch_create(&ft_data.contract_id);

            // Pay the required storage as outlined in the AccountData. This will run first and then we send the fungible tokens
            // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "storage_deposit",
                json!({ "account_id": account_id }).to_string().as_bytes(),
                ft_data.ft_storage.0,
                MIN_GAS_FOR_STORAGE_DEPOSIT,
                GasWeight(1),
            );

            // Send the fungible tokens (after the storage attached_deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "ft_transfer",
                json!({ "receiver_id": account_id, "amount": ft_data.balance_per_use, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                1,
                MIN_GAS_FOR_FT_TRANSFER,
                GasWeight(1)
            );

            // Create the second batch promise to execute after the batch_ft_promise_id batch is finished executing.
            // It will execute on the current account ID (this contract)
            let batch_ft_resolve_promise_id =
                env::promise_batch_then(batch_ft_promise_id, &env::current_account_id());

            // Execute a function call as part of the resolved promise index created in promise_batch_then
            // Callback after both the storage was deposited and the fungible tokens were sent
            // Call the function with the min GAS and then attach 3/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_resolve_promise_id,
                "ft_resolve_batch",
                json!({ "amount": ft_data.balance_per_use, "token_sender": ft_data.sender_id, "token_contract": ft_data.contract_id }).to_string().as_bytes(),
                NO_DEPOSIT,
                MIN_GAS_FOR_RESOLVE_BATCH,
                GasWeight(3)
            );
        } else {
            // Create a new batch promise to pay storage and refund the FTs to the original sender
            let batch_ft_promise_id = env::promise_batch_create(&ft_data.contract_id);

            // Send the fungible tokens (after the storage attached_deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "storage_deposit",
                json!({ "account_id": ft_data.sender_id })
                    .to_string()
                    .as_bytes(),
                ft_data.ft_storage.0,
                MIN_GAS_FOR_STORAGE_DEPOSIT,
                GasWeight(1),
            );

            // Send the fungible tokens (after the storage attached_deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "ft_transfer",
                json!({ "receiver_id": ft_data.sender_id, "amount": ft_data.balance_per_use, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                1,
                MIN_GAS_FOR_FT_TRANSFER,
                GasWeight(1)
            );

            // Return the result of the batch as the return of the function
            env::promise_return(batch_ft_promise_id);
        }
    }
}
