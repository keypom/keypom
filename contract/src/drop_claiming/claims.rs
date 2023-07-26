use crate::*;
use near_sdk::ext_contract;

/// Interface for account creation
#[ext_contract(ext_account_creation)]
trait ExtAccountCreation {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}

#[near_bindgen]
impl Keypom {
    pub fn claim(&mut self, account_id: AccountId, fc_args: UserProvidedFCArgs) -> Promise {
        let mut event_logs = Vec::new();
        let (token_id, required_asset_gas) = self.before_claim_logic(
            &mut event_logs,
            &account_id,
            None
        );

        let prepaid_gas = env::prepaid_gas();
        let total_required_gas = BASE_GAS_FOR_CLAIM + required_asset_gas;
        require!(
            prepaid_gas == total_required_gas,
            format!("Not enough gas attached. Required: {}, Prepaid: {}",
            total_required_gas.0,
            prepaid_gas.0)
        );

        self.internal_claim_assets(token_id, account_id, fc_args)
    }

    pub fn create_account_and_claim(&mut self, new_account_id: AccountId, new_public_key: PublicKey, fc_args: UserProvidedFCArgs) -> Promise {
        let mut event_logs = Vec::new();
        let (token_id, required_asset_gas) = self.before_claim_logic(
            &mut event_logs,
            &new_account_id,
            Some(&new_public_key)
        );

        let prepaid_gas = env::prepaid_gas();
        let total_required_gas = BASE_GAS_FOR_CREATE_ACC_AND_CLAIM + required_asset_gas;
        require!(
            prepaid_gas == total_required_gas,
            format!("Not enough gas attached. Required: {}, Prepaid: {}",
            total_required_gas.0,
            prepaid_gas.0)
        );
        let gas_for_callback = BASE_GAS_FOR_RESOLVE_ACCOUNT_CREATION + total_required_gas;
        
        // First, create the zero-balance account and then, claim the assets
        ext_account_creation::ext(self.root_account.clone())
            .with_static_gas(GAS_FOR_CREATE_ACCOUNT)
            .with_unused_gas_weight(0)
            .with_attached_deposit(10000000000000000000000) // TODO: remove (needed for sandbox testing)
            .create_account(
                new_account_id.clone(),
                new_public_key
        ).then(
            // Call resolve refund with the min GAS and no attached_deposit. 1/2 unspent GAS will be added on top
            Self::ext(env::current_account_id())
                .with_static_gas(gas_for_callback)
                .with_unused_gas_weight(1)
                .on_new_account_created(
                    token_id,
                    new_account_id,
                    fc_args
                )
        )
    }
}