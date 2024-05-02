use crate::*;

use serde_json::Value;

#[near_bindgen]
impl Keypom {
    /// Allows users to add to their balance. This is to prepay and cover drop costs
    #[payable]
    pub fn set_funder_metadata(&mut self, metadata: Option<String>) -> bool {
        self.assert_no_global_freeze();
        let refund_amount =
            self.internal_modify_user_metadata(metadata, env::attached_deposit().as_yoctonear(), None);

        if refund_amount > 0 {
            Promise::new(env::predecessor_account_id().clone())
                .transfer(NearToken::from_yoctonear(refund_amount))
                .as_return();
        }

        true
    }

    /// Allows users to add to their balance. This is to prepay and cover drop costs
    #[payable]
    pub fn add_to_balance(&mut self) -> bool {
        self.assert_no_global_freeze();

        // Get the attached_deposit value which is how much the user wants to add to their storage
        let attached_deposit = env::attached_deposit().as_yoctonear();

        self.internal_modify_user_balance(&env::predecessor_account_id(), attached_deposit, false);

        true
    }

    /// Allows users to withdraw their balance
    pub fn withdraw_from_balance(&mut self, amount_to_withdraw: Option<U128>) -> bool {
        self.assert_no_global_freeze();

        // The account to withdraw storage to is always the predecessor
        let owner_id = env::predecessor_account_id();
        // Get the amount that the user has by removing them from the map. If they're not in the map, default to 0
        // Possible re-entrency attack if we don't remove the user from the map before transferring
        let mut funder_info = self
            .funder_info_by_id
            .remove(&owner_id)
            .expect("User not found");
        let mut cur_balance: u128 = funder_info.balance;

        // If the amount to withdraw is specified, we withdraw the min of the amount to withdraw and the amount the user has
        let amount = amount_to_withdraw.map(|x| x.0).unwrap_or(cur_balance);
        if amount <= cur_balance && amount > 0 {
            // decrement the balance by the amount withdrawn
            cur_balance -= amount;

            Promise::new(owner_id.clone())
                .transfer(NearToken::from_yoctonear(amount))
                .as_return();
        }

        // re-insert the balance into the map if it's greater than 0
        funder_info.balance = cur_balance;
        self.funder_info_by_id.insert(&owner_id, &funder_info);

        true
    }

    /// Return the current balance for a given account
    pub fn get_user_balance(&self, account_id: AccountId) -> U128 {
        U128(
            self.funder_info_by_id
                .get(&account_id)
                .map(|x| x.balance)
                .unwrap_or(0),
        )
    }

    /// Internal function to modify the user's balance. Defaults to adding the amount but decrement can also be specified
    pub(crate) fn internal_modify_user_balance(
        &mut self,
        account_id: &AccountId,
        amount: u128,
        decrement: bool,
    ) {
        if amount > 0 {
            // Get the balance of the account (if the account isn't in the map we default to a balance of 0)
            let mut funder_info = self
                .funder_info_by_id
                .get(account_id)
                .unwrap_or(FunderInfo {
                    metadata: None,
                    balance: 0,
                });
            let mut balance: u128 = funder_info.balance;

            // Either add or subtract the amount from the balance depending on whether or not decrement was passed in
            if decrement {
                require!(
                    balance >= amount,
                    format!("User balance {} is less than required {}", balance, amount)
                );
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

            funder_info.balance = balance;
            // Insert the balance back into the map for that account ID
            self.funder_info_by_id.insert(account_id, &funder_info);
        }
    }

    /// Checks that the attached deposit is greater than the required deposit and refunds any excess
    /// If the attached deposit is less than the required deposit, decrement the user balance
    /// If the user doesn't have enough deposit or balance, panic
    pub(crate) fn charge_with_deposit_or_balance(
        &mut self,
        required_deposit: Balance,
        attached_deposit: Balance,
    ) -> Balance {
        let predecessor = env::predecessor_account_id();
        let can_deposit_cover = attached_deposit >= required_deposit;

        // In the case that the attached deposit covers what is required, refund the excess
        if can_deposit_cover {
            let amount_to_refund = attached_deposit - required_deposit;
            return amount_to_refund;
        }

        // In the case that the attached deposit is less than the required, check user balance
        let required_deposit_left = required_deposit - attached_deposit;
        self.internal_modify_user_balance(&predecessor, required_deposit_left, true);
        return 0;
    }

    /// Internal function to change the user's metadata and return how much this operation cost (in
    /// terms of raw $NEAR. If they didn't attach enough but their user balance covers, this
    /// function will charge their balance and return 0)
    /*
     * NET RELEASE
     * - User attached some (return new balance and refund)
     * - User attached none (return new balance and refund)
     *
     * NET ZERO
     * - User attached some (return that same some and refund)
     * - User attached none (return none and don't refund)
     *
     * NET CHARGE
     * - User attached enough (return the net and refund)
     * - User attached not enough
     *   - User has enough balance (return 0 and don't refund)
     *   - User has not enough balance (panick)
     */
    pub(crate) fn internal_modify_user_metadata(
        &mut self,
        new_metadata: Option<String>,
        attached_deposit: Balance,
        append_to_metadata: Option<bool>,
    ) -> Balance {
        let caller_id = env::predecessor_account_id();

        let initial_storage = env::storage_usage();
        let mut funder_info = self
            .funder_info_by_id
            .get(&caller_id)
            .unwrap_or(FunderInfo {
                metadata: None,
                balance: 0,
            });
        // Check if new_metadata is valid JSON
        if let Some(metadata_str) = &new_metadata {
            let result = serde_json::from_str::<serde_json::Value>(metadata_str);
            require!(result.is_ok(), format!("New funder metadata is not valid JSON: {:?}", result.as_ref().err().unwrap()).to_string());
        }

        // Overwrite if specified, otherwise, append (default)
        if append_to_metadata.unwrap_or(true) {
            // Append to existing metadata
            near_sdk::log!("Existing Metadata: {:?}", funder_info.metadata);
            if let Some(existing_metadata) = &funder_info.metadata {
                let modified_existing_metadata = &existing_metadata[..&existing_metadata.len() - 1];
                let modified_input_metadata = &new_metadata.unwrap()[1..];
                let new_metadata = format!("{},{}", modified_existing_metadata, modified_input_metadata);

                // ensure new metadata is still JSON
                let result = serde_json::from_str::<serde_json::Value>(&new_metadata);
                require!(result.is_ok(), format!("New funder metadata is not valid JSON: {:?}", result.as_ref().err().unwrap()).to_string());

                funder_info.metadata = Some(new_metadata);
            } else {
                // Old metadata was empty
                funder_info.metadata = new_metadata;
            }
        } else {
            // Overwrite current metadata
            funder_info.metadata = new_metadata;
            
        }


        self.funder_info_by_id.insert(&caller_id, &funder_info);
        
        let final_storage = env::storage_usage();

        let mut refund_amount = attached_deposit;

        // We used more storage, so we should charge the user
        if final_storage > initial_storage {
            let storage_cost =
                (final_storage - initial_storage) as u128 * env::storage_byte_cost().as_yoctonear();
            near_sdk::log!(
                "Charging user for storage: {} (deposit: {})",
                storage_cost,
                refund_amount
            );

            // If the user doesn't have enough attached deposit, try to decrement the user balance for whatever is less
            if attached_deposit < storage_cost {
                self.internal_modify_user_balance(
                    &caller_id,
                    storage_cost - attached_deposit,
                    true,
                );
                refund_amount = 0;
            } else {
                refund_amount -= storage_cost;
            }

        // We used less storage, so we should refund the user for whatever attached deposit they
        // had plus the storage refund
        } else if final_storage <= initial_storage {
            let storage_refund =
                (initial_storage - final_storage) as u128 * env::storage_byte_cost().as_yoctonear();
            refund_amount += storage_refund;
        }

        // Return the refund amount
        refund_amount
    }
}