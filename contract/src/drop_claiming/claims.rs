use crate::*;

#[near_bindgen]
impl Keypom {
    #[private]
    pub fn claim(
        &mut self,
        account_id: AccountId,
        fc_args: UserProvidedFCArgs,
        password: Option<String>,
    ) -> PromiseOrValue<bool> {
        self.assert_no_global_freeze();

        let mut event_logs: Vec<EventLog> = Vec::new();
        let BeforeClaimData {
            token_id,
            required_asset_gas,
            root_account_id: _,
            account_creation_keypom_args: _,
            drop_id: _,
            key_id: _,
            funder_id: _,
        } = self.before_claim_logic(&mut event_logs, None, password);
        let prepaid_gas = env::prepaid_gas();
        let total_required_gas = BASE_GAS_FOR_CLAIM.as_gas() + required_asset_gas.as_gas();
        // Use to check prepaid == required. Changed to >= for the sake of simplicity for now
        // Future plan to implement required_gas_for_claim and required_gas_for_create_account_and_claim into key info
        require!(
            prepaid_gas.as_gas() >= total_required_gas,
            format!(
                "Not enough gas attached. Required: {}, Prepaid: {}",
                total_required_gas,
                prepaid_gas.as_gas()
            )
        );

        log_events(event_logs);
        self.internal_claim_assets(token_id, account_id, fc_args, None)
    }

    #[private]
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: AccountId,
        new_public_key: PublicKey,
        fc_args: UserProvidedFCArgs,
        password: Option<String>,
    ) -> Promise {
        self.assert_no_global_freeze();

        let mut event_logs = Vec::new();
        let BeforeClaimData {
            token_id,
            required_asset_gas,
            root_account_id,
            account_creation_keypom_args,
            drop_id,
            key_id,
            funder_id,
        } = self.before_claim_logic(&mut event_logs, Some(&new_public_key), password);

        let prepaid_gas = env::prepaid_gas();
        let total_required_gas =
            BASE_GAS_FOR_CREATE_ACC_AND_CLAIM.as_gas() + required_asset_gas.as_gas();
        require!(
            prepaid_gas.as_gas() >= total_required_gas,
            format!(
                "Not enough gas attached. Required: {}, Prepaid: {}",
                total_required_gas,
                prepaid_gas.as_gas()
            )
        );
        let gas_for_callback =
            BASE_GAS_FOR_RESOLVE_ACCOUNT_CREATION.as_gas() + required_asset_gas.as_gas();
        near_sdk::log!("gas_for_callback: {}", gas_for_callback);

        log_events(event_logs);
        near_sdk::log!(
            "Keypom Args Before create_account: {:?}",
            account_creation_keypom_args
        );
        let mut create_account_args = json!({
            "new_account_id": new_account_id,
            "new_public_key": new_public_key
        })
        .to_string();

        add_keypom_args(
            &mut create_account_args,
            account_creation_keypom_args,
            &new_account_id,
            &drop_id,
            &key_id.to_string(),
            &funder_id,
        )
        .expect("Unable to add keypom args");

        // First, create the zero-balance account and then, claim the assets

        Promise::new(root_account_id)
            .function_call_weight(
                "create_account".to_string(),
                create_account_args.into(),
                NearToken::from_yoctonear(0),
                GAS_FOR_CREATE_ACCOUNT,
                GasWeight(0),
            )
            .then(
                // Call resolve refund with the min GAS and no attached_deposit. 1/2 unspent GAS will be added on top
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_gas(gas_for_callback))
                    .with_unused_gas_weight(1)
                    .on_new_account_created(token_id, new_account_id, fc_args, new_public_key),
            )
    }
}

