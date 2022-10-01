use crate::*;

#[near_bindgen]
impl Keypom {
    /// Add a prohibited method to the list of methods that can't be called by a FC Drop
    #[payable]
    pub fn register_ft_contract(&mut self, account_id: AccountId) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        self.internal_register_ft_contract(&account_id, env::attached_deposit(), &env::predecessor_account_id(), false);
        self.registered_ft_contracts.insert(&account_id);
    }

    /// Add a prohibited method to the list of methods that can't be called by a FC Drop
    pub fn add_prohibited_method(&mut self, method: String) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        self.prohibited_fc_methods.insert(&method);
    }

    /// Set the desired linkdrop contract to interact with
    pub fn set_root_account(&mut self, root_account: AccountId) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        self.root_account = root_account;
    }

    /// Set the desired contract fees
    pub fn set_fees(&mut self, drop_fee: Option<U128>, key_fee: Option<U128>) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        if let Some(drop_fee) = drop_fee {
            self.drop_fee = drop_fee.0;
        }
        if let Some(key_fee) = key_fee {
            self.key_fee = key_fee.0;
        }
    }

    /// Set the desired linkdrop contract to interact with
    pub fn set_fees_per_user(&mut self, account_id: AccountId, drop_fee: U128, key_fee: U128) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "predecessor != owner"
        );
        self.fees_per_user
            .insert(&account_id, &(drop_fee.0, key_fee.0));
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
