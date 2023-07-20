use crate::*;

/// Which methods are prohibited from being called by an FC drop
pub const DEFAULT_PROHIBITED_FC_METHODS: [&str; 6] = [
    "nft_transfer",
    "nft_transfer_call",
    "nft_approve",
    "nft_transfer_payout",
    "ft_transfer",
    "ft_transfer_call",
];

/// Gas needed to execute any logic in the fc claim function
/// 2 TGas
pub const GAS_FOR_FC_CLAIM_LOGIC: Gas = Gas(2_000_000_000_000);

impl FCData {
    /// Loop through each method and create a promise to call the method
    /// Each of these methods will be executed 1 after the next.
    /// If anything goes wrong, there's nothing the contract will do to refund.
    pub fn claim_fc_asset(
        &mut self,
        fc_args: AssetSpecificFCArgs,
        account_id: AccountId,
        drop_id: DropId,
        key_id: String,
        funder_id: AccountId
    ) -> Promise {  
        let mut promises = Vec::new();

        for (idx, method) in self.methods.iter().enumerate() {
            let mut actual_args = method.args.clone();
            
            match handle_fc_args(
                &mut actual_args, 
                method.keypom_args.clone(), 
                &method.user_args_rule, 
                &fc_args, 
                &account_id, 
                &drop_id, 
                &key_id, 
                &funder_id, 
                idx
            ) {
                Ok(_) => {},
                Err(e) => {
                    near_sdk::log!("Error handling FC args: {:?}", e);
                    continue;
                }
            }

            let promise = Promise::new(method.receiver_id.clone())
                .function_call_weight(
                    method.method_name.clone(), 
                    method.args.clone().into(), 
                    method.attached_deposit.0,
                    method.attached_gas,
                    GasWeight(1)
                );
            promises.push(promise);
        }

        promises.into_iter().reduce(|a, b| a.then(b)).expect("empty promises")
    }
}