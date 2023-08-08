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
/// For every length of the args, add this much gas
pub const GAS_PER_ARG_LENGTH: Gas = Gas(400_000_000);

impl FCData {
    /// Loop through each method and create a promise to call the method
    /// Each of these methods will be executed 1 after the next.
    /// If anything goes wrong, there's nothing the contract will do to refund.
    pub fn claim_fc_asset(
        &mut self,
        mut fc_args: AssetSpecificFCArgs,
        account_id: AccountId,
        drop_id: DropId,
        key_id: String,
        funder_id: AccountId
    ) -> Option<Promise> {  
        let mut promises = Vec::new();
        let num_fc_args = fc_args.as_ref().and_then(|a| Some(a.len())).unwrap_or(self.methods.len());
        if num_fc_args != self.methods.len() {
            near_sdk::log!("User provided FC args does not match methods length. Found {} Expected {} Skipping all FC args.", num_fc_args, self.methods.len());
            fc_args = None;
        }

        for (idx, method) in self.methods.iter().enumerate() {
            let mut actual_args = method.args.clone();
            if actual_args.len() == 0 {
                actual_args = "{}".to_string();
            }
            
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

            let receiver_id = match determine_receiver_id(
                &method.receiver_to_claimer,
                &method.receiver_id,
                &account_id
            ) {
                Ok(receiver_id) => receiver_id,
                Err(e) => {
                    near_sdk::log!("Error determining receiver ID: {:?}", e);
                    continue;
                }
            };

            near_sdk::log!("Final Receiver ID: {}", receiver_id);
            near_sdk::log!("Final Method Name: {}", method.method_name);
            //near_sdk::log!("Final Args: {}", actual_args);
            near_sdk::log!("Final Attached Deposit: {}", method.attached_deposit.0);
            near_sdk::log!("Final Attached Gas: {}", method.attached_gas.0);
            
            let promise = Promise::new(receiver_id)
                .function_call_weight(
                    method.method_name.clone(), 
                    actual_args.into(), 
                    method.attached_deposit.0,
                    method.attached_gas,
                    GasWeight(1)
                );
            promises.push(promise);
        }

        promises.into_iter().reduce(|a, b| a.then(b))
    }
}