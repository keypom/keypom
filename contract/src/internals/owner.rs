use crate::*;

#[near_bindgen]
impl Keypom {
    /// Set the desired linkdrop contract to interact with
    pub fn set_root_account(&mut self, root_account: AccountId) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        self.root_account = root_account;
    }

    /// Set the desired linkdrop contract to interact with
    pub fn set_gas_price(&mut self, yocto_per_gas: u128) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        self.yocto_per_gas = yocto_per_gas;
    }

    /// Withdraw the fees collected to the passed in Account Id
    pub fn withdraw_fees(&mut self, withdraw_to: AccountId) -> Promise {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        let amount = self.fees_collected;
        self.fees_collected = 0;

        Promise::new(withdraw_to)
            .transfer(amount)
            .then(Self::ext(env::current_account_id()).on_withdraw_fees(amount))
    }

    /// Callback for withdrawing fees on the contract
    #[private]
    pub fn on_withdraw_fees(&mut self, fees_collected: u128) -> bool {
        let result = promise_result_as_success();

        // If something went wrong, set the fees collected again
        if result.is_none() {
            self.fees_collected += fees_collected;
            return false;
        }

        true
    }
}
