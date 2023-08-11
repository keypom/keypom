use crate::*;
use near_sdk::ext_contract;

/// Interface for account creation
#[ext_contract(ext_account_creation)]
trait ExtAccountCreation {
    fn create_account(
        &mut self, 
        new_account_id: AccountId, 
        new_public_key: PublicKey,
        keypom_args: Option<KeypomInjectedArgs>
    ) -> Promise;
}

#[near_bindgen]
impl Keypom {
    #[private]
    pub fn claim(&mut self, account_id: AccountId, fc_args: UserProvidedFCArgs, password: Option<String>) -> PromiseOrValue<bool> {
        self.assert_no_global_freeze();

        let mut event_logs: Vec<EventLog> = Vec::new();
        let initial_gas = env::used_gas();
        let BeforeClaimData { token_id, required_asset_gas, root_account_id: _, account_creation_keypom_args: _ } = self.before_claim_logic(
            &mut event_logs,
            &account_id,
            None,
            password
        );
        near_sdk::log!("Gas for before claim {}", (env::used_gas() - initial_gas).0);

        let prepaid_gas = env::prepaid_gas();
        let total_required_gas = BASE_GAS_FOR_CLAIM + required_asset_gas;
        // Use to check prepaid == required. Changed to >= for the sake of simplicity for now
        // Future plan to implement required_gas_for_claim and required_gas_for_create_account_and_claim into key info
        require!(
            prepaid_gas >= total_required_gas,
            format!("Not enough gas attached. Required: {}, Prepaid: {}",
            total_required_gas.0,
            prepaid_gas.0)
        );

        log_events(event_logs);
        self.internal_claim_assets(token_id, account_id, fc_args, None)
    }

    #[private]
    pub fn create_account_and_claim(&mut self, new_account_id: AccountId, new_public_key: PublicKey, fc_args: UserProvidedFCArgs, password: Option<String>) -> Promise {
        self.assert_no_global_freeze();

        let mut event_logs = Vec::new();
        let BeforeClaimData { token_id, required_asset_gas, root_account_id, account_creation_keypom_args } = self.before_claim_logic(
            &mut event_logs,
            &new_account_id,
            Some(&new_public_key),
            password
        );

        let prepaid_gas = env::prepaid_gas();
        let total_required_gas = BASE_GAS_FOR_CREATE_ACC_AND_CLAIM + required_asset_gas;
        require!(
            prepaid_gas >= total_required_gas,
            format!("Not enough gas attached. Required: {}, Prepaid: {}",
            total_required_gas.0,
            prepaid_gas.0)
        );
        let gas_for_callback = BASE_GAS_FOR_RESOLVE_ACCOUNT_CREATION + required_asset_gas;
        near_sdk::log!("gas_for_callback: {}", gas_for_callback.0);

        log_events(event_logs);
        near_sdk::log!("Keypom Args Before create_account: {:?}", account_creation_keypom_args);
        // First, create the zero-balance account and then, claim the assets
        ext_account_creation::ext(root_account_id)
            .with_static_gas(GAS_FOR_CREATE_ACCOUNT)
            .with_unused_gas_weight(0)
            .with_attached_deposit(10000000000000000000000) // TODO: remove (needed for sandbox testing)
            .create_account(
                new_account_id.clone(),
                new_public_key.clone(),
                account_creation_keypom_args
        ).then(
            // Call resolve refund with the min GAS and no attached_deposit. 1/2 unspent GAS will be added on top
            Self::ext(env::current_account_id())
                .with_static_gas(gas_for_callback)
                .with_unused_gas_weight(1)
                .on_new_account_created(
                    token_id,
                    new_account_id,
                    fc_args,
                    new_public_key
                )
        )
    }
}