use crate::*;
    

#[near_bindgen]
impl DropZone {
    /// Set the desired linkdrop contract to interact with
    pub fn set_contract(&mut self, linkdrop_contract: AccountId) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        self.linkdrop_contract = linkdrop_contract;
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

        Promise::new(withdraw_to).transfer(amount).then(
            Self::ext(env::current_account_id())
                .on_withdraw_fees(amount)
        )
    }

    /// Callback for withdrawing fees on the contract
    #[private]
    pub fn on_withdraw_fees(&mut self, fees_collected: u128) -> bool {
        let result = promise_result_as_success();

        // If something went wrong, set the fees collected again
        if result.is_none() {
            self.fees_collected += fees_collected;
            return false
        }

        true
    }
}