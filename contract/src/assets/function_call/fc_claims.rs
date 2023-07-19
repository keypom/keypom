use crate::*;

/// Gas needed to execute any logic in the ft claim function
/// 2 TGas + 2 * CCC gas (since there are 2 CCCs)
/// 12 TGas
pub const GAS_FOR_FC_CLAIM_LOGIC: Gas = Gas(2_000_000_000_000 + 2 * MIN_BASE_GAS_FOR_ONE_CCC.0);

impl FCData {
    /// Attempt to transfer FTs to a given address (will cover registration automatically).
    /// If the transfer fails, the FTs will be returned to the available balance
    /// Should *only* be invoked if the available balance is greater than or equal to the transfer amount.
    pub fn claim_fc_asset(&mut self) -> Promise {  
        let mut promises = Vec::new();

        for method in self.methods.iter() {
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