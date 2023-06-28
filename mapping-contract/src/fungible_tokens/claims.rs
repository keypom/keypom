use near_sdk::{Gas, GasWeight, serde_json::json, PromiseResult, require};

use crate::*;

/// Gas needed to execute any logic in the ft claim function
/// 2 TGas + 3 * CCC gas (since there are 3 CCCs)
/// 17 TGas
pub const GAS_FOR_CLAIM_LOGIC: Gas = Gas(2_000_000_000_000 + 3 * MIN_BASE_GAS_FOR_ONE_CCC.0);
/// Minimum Gas required to perform a simple transfer of fungible tokens.
/// 5 TGas
pub const MIN_GAS_FOR_FT_TRANSFER: Gas = Gas(5_000_000_000_000);
/// Minimum Gas required to register a user on the FT contract
/// 5 TGas
pub const MIN_GAS_FOR_STORAGE_DEPOSIT: Gas = Gas(5_000_000_000_000);
/// Minimum Gas required to resolve the batch of promises for transferring the FTs and registering the user.
/// 5 TGas
pub const MIN_GAS_FOR_RESOLVE_BATCH: Gas = Gas(5_000_000_000_000);

impl InternalFTData {
    /// Attempt to transfer FTs to a given address (will cover registration automatically).
    /// If the transfer fails, the FTs will be returned to the available balance
    /// Should *only* be invoked if the available balance is greater than or equal to the transfer amount.
    pub fn claim_ft_asset(&mut self, drop_id: &DropId, receiver_id: &AccountId, transfer_amount: &u128) {
        near_sdk::log!("ft_claim receiver: {} amount: {}", receiver_id, transfer_amount);

        let mut gas_used = env::used_gas();
        require!(self.enough_balance(&transfer_amount), "not enough balance to transfer");
        // Decrement the available balance and then invoke the transfer
        self.balance_avail -= transfer_amount;

        // Create a new batch promise to pay storage and transfer FTs to the new account ID
        let batch_transfer = env::promise_batch_create(&self.contract_id);
        near_sdk::log!("gas used 11{:?}", env::used_gas() - gas_used);
        gas_used = env::used_gas();

        // Pay the required storage as outlined in the AccountData. This will run first and then we send the fungible tokens
        // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
        env::promise_batch_action_function_call_weight(
            batch_transfer,
            "storage_deposit",
            json!({ "account_id": receiver_id }).to_string().as_bytes(),
            self.registration_cost,
            MIN_GAS_FOR_STORAGE_DEPOSIT,
            GasWeight(1),
        );
        near_sdk::log!("gas used 12{:?}", env::used_gas() - gas_used);
        gas_used = env::used_gas();

        // Send the fungible tokens (after the storage attached_deposit is finished since these run sequentially)
        // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
        env::promise_batch_action_function_call_weight(
            batch_transfer,
            "ft_transfer",
            json!({ "receiver_id": receiver_id, "amount": transfer_amount.to_string(), "memo": "Keypom FT Tokens" }).to_string().as_bytes(),
            1,
            MIN_GAS_FOR_FT_TRANSFER,
            GasWeight(1)
        );
        near_sdk::log!("gas used 13{:?}", env::used_gas() - gas_used);
        gas_used = env::used_gas();

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
            json!({ "amount": transfer_amount.to_string(), "drop_id": drop_id, "data_id": self.get_asset_id() }).to_string().as_bytes(),
            0,
            MIN_GAS_FOR_RESOLVE_BATCH,
            GasWeight(3)
        );
        near_sdk::log!("gas used 14{:?}", env::used_gas() - gas_used);
    }

    /// Private function that will be called after the FT claim is finished. This will check whether the claim went through successfully.
    /// If it was unsuccessful, the available balance will be incremented (acting as a refund that can then be claimed via refund method)
    fn resolve_ft_claim(&mut self, transfer_amount: u128) -> bool {
        // check whether or not the transfer was successful
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        if transfer_succeeded {
            return true;
        }

        // If the transfer failed, then we need to increment the available balance
        self.balance_avail += transfer_amount;

        false
    }

    /// Check how much gas is required for a simple FT claim 
    pub fn get_min_required_gas(&self) -> Gas {
        GAS_FOR_CLAIM_LOGIC + MIN_GAS_FOR_FT_TRANSFER + MIN_GAS_FOR_STORAGE_DEPOSIT + MIN_GAS_FOR_RESOLVE_BATCH
    }
}