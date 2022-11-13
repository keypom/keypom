use crate::*;

#[near_bindgen]
impl Keypom {
    // Allows users to add to their balance. This is to prepay and cover drop costs
    #[payable]
    pub fn add_to_balance(&mut self) {
        // Get the attached_deposit value which is how much the user wants to add to their storage
        let attached_deposit = env::attached_deposit();

        self.internal_modify_user_balance(&env::predecessor_account_id(), attached_deposit, false);
    }

    // Allows users to withdraw their balance
    pub fn withdraw_from_balance(&mut self) {
        // the account to withdraw storage to is always the predecessor
        let owner_id = env::predecessor_account_id();
        //get the amount that the user has by removing them from the map. If they're not in the map, default to 0
        let amount = self.user_balances.remove(&owner_id).unwrap_or(0);

        //if that excess to withdraw is > 0, we transfer the amount to the user.
        if amount > 0 {
            Promise::new(owner_id.clone()).transfer(amount);
        }
    }

    // Return the current balance for a given account
    pub fn get_user_balance(&self, account_id: AccountId) -> U128 {
        U128(self.user_balances.get(&account_id).unwrap_or(0))
    }
}
