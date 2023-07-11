use crate::*;

#[near_bindgen]
impl Keypom {
    /// Allows users to add to their balance. This is to prepay and cover drop costs
    #[payable]
    pub fn add_to_balance(&mut self) {
        // Get the attached_deposit value which is how much the user wants to add to their storage
        let attached_deposit = env::attached_deposit();

        self.internal_modify_user_balance(&env::predecessor_account_id(), attached_deposit, false);
    }

    /// Allows users to withdraw their balance
    pub fn withdraw_from_balance(&mut self, amount_to_withdraw: Option<U128>) {
        // The account to withdraw storage to is always the predecessor
        let owner_id = env::predecessor_account_id();
        // Get the amount that the user has by removing them from the map. If they're not in the map, default to 0
        // Possible re-entrency attack if we don't remove the user from the map before transferring
        let mut cur_balance = self.user_balances.remove(&owner_id).unwrap_or(0);

        // If the amount to withdraw is specified, we withdraw the min of the amount to withdraw and the amount the user has
        let amount = amount_to_withdraw.map(|x| x.0).unwrap_or(cur_balance);
        if amount <= cur_balance && amount > 0 {
            // decrement the balance by the amount withdrawn
            cur_balance -= amount;

            Promise::new(owner_id.clone()).transfer(amount);
        }

        // re-insert the balance into the map if it's greater than 0
        if cur_balance > 0 {
            self.user_balances.insert(&owner_id, &cur_balance);
        }
    }

    /// Return the current balance for a given account
    pub fn get_user_balance(&self, account_id: AccountId) -> U128 {
        U128(self.user_balances.get(&account_id).unwrap_or(0))
    }

    /// Internal function to modify the user's balance. Defaults to adding the amount but decrement can also be specified
    pub(crate) fn internal_modify_user_balance(&mut self, account_id: &AccountId, amount: u128, decrement: bool) {
        if amount > 0 {
            // Get the balance of the account (if the account isn't in the map we default to a balance of 0)
            let mut balance: u128 = self
                .user_balances
                .get(account_id)
                .unwrap_or(0);
    
            // Either add or subtract the amount from the balance depending on whether or not decrement was passed in
            if decrement == true {
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
    
            // Insert the balance back into the map for that account ID
            self.user_balances
                .insert(account_id, &balance);
        }
    }


    /// Checks that the attached deposit is greater than the required deposit and refunds any excess
    /// If the attached deposit is less than the required deposit, decrement the user balance
    /// If the user doesn't have enough deposit or balance, panic
    pub(crate) fn charge_with_deposit_or_balance(&mut self, required_deposit: Balance) {
        let predecessor = env::predecessor_account_id();
        let can_deposit_cover = env::attached_deposit() >= required_deposit;
        
        // In the case that the attached deposit covers what is required, refund the excess
        if can_deposit_cover {
            let amount_to_refund = env::attached_deposit() - required_deposit;

            near_sdk::log!("Refunding {} excess deposit", amount_to_refund);
            Promise::new(predecessor).transfer(amount_to_refund);
            return;
        }

        // In the case that the attached deposit is less than the required, check user balance
        let required_deposit_left = required_deposit - env::attached_deposit();
        self.internal_modify_user_balance(&predecessor, required_deposit_left, true);
    }
}