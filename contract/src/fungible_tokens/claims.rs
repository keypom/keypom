use near_sdk::{Gas, GasWeight, serde_json::json, PromiseResult};

use crate::*;

/// Minimum Gas required to perform a simple transfer of fungible tokens.
/// 5 TGas
const MIN_GAS_FOR_FT_TRANSFER: Gas = Gas(5_000_000_000_000);
/// Minimum Gas required to register a user on the FT contract
/// 5 TGas
const MIN_GAS_FOR_STORAGE_DEPOSIT: Gas = Gas(5_000_000_000_000);
/// Minimum Gas required to resolve the batch of promises for transferring the FTs and registering the user.
/// 5 TGas
const MIN_GAS_FOR_RESOLVE_BATCH: Gas = Gas(5_000_000_000_000);

impl InternalFTData {
    /// Attempt to transfer FTs to a given address (will cover registration automatically).
    /// If the transfer fails, the FTs will be returned to the available balance
    /// Should *only* be invoked if the available balance is greater than or equal to the transfer amount.
    pub fn ft_claim(&mut self, drop_id: U128, receiver_id: AccountId, transfer_amount: U128) {
        // Decrement the available balance and then invoke the transfer
        self.balance_avail.0 -= transfer_amount.0;

        // Create a new batch promise to pay storage and transfer FTs to the new account ID
        let batch_transfer = env::promise_batch_create(&self.contract_id);

        // Pay the required storage as outlined in the AccountData. This will run first and then we send the fungible tokens
        // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
        env::promise_batch_action_function_call_weight(
            batch_transfer,
            "storage_deposit",
            json!({ "account_id": receiver_id }).to_string().as_bytes(),
            self.registration_cost.0,
            MIN_GAS_FOR_STORAGE_DEPOSIT,
            GasWeight(1),
        );

        // Send the fungible tokens (after the storage attached_deposit is finished since these run sequentially)
        // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
        env::promise_batch_action_function_call_weight(
            batch_transfer,
            "ft_transfer",
            json!({ "receiver_id": receiver_id, "amount": transfer_amount.0, "memo": "Keypom FT Tokens" }).to_string().as_bytes(),
            1,
            MIN_GAS_FOR_FT_TRANSFER,
            GasWeight(1)
        );

        // Create the second batch promise to execute after the storage & tokens were transferred
        // It will execute on the current account ID (this contract)
        let batch_resolve =
            env::promise_batch_then(batch_transfer, &env::current_account_id());

        // Execute a function call as part of the resolved promise index created in promise_batch_then
        // Callback after both the storage was deposited and the fungible tokens were sent
        // Call the function with the min GAS and then attach 3/5 of the unspent GAS to the call
        env::promise_batch_action_function_call_weight(
            batch_resolve,
            "ft_resolve_batch",
            json!({ "amount": transfer_amount, "drop_id": drop_id, "data_id": self.get_data_id() }).to_string().as_bytes(),
            0,
            MIN_GAS_FOR_RESOLVE_BATCH,
            GasWeight(3)
        );
    }

    /// Private function that will be called after the FT claim is finished. This will check whether the claim went through successfully.
    /// If it was unsuccessful, the available balance will be incremented (acting as a refund that can then be claimed via refund method)
    pub fn resolve_ft_claim(&mut self, transfer_amount: U128) -> bool {
        // check whether or not the transfer was successful
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        if transfer_succeeded {
            return true;
        }

        // If the transfer failed, then we need to increment the available balance
        self.balance_avail.0 += transfer_amount.0;
        
        false
    }
}