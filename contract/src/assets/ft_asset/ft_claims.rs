use near_sdk::{serde_json::json, Gas, GasWeight};

use crate::*;

/// Gas needed to execute any logic in the ft claim function
/// 2 TGas + 2 * CCC gas (since there are 2 CCCs)
/// 12 TGas
pub const GAS_FOR_FT_CLAIM_LOGIC: Gas = Gas::from_tgas(9);
/// Minimum Gas required to perform a simple transfer of fungible tokens.
/// 5 TGas
pub const MIN_GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(5);
/// Minimum Gas required to register a user on the FT contract
/// 5 TGas
pub const MIN_GAS_FOR_STORAGE_DEPOSIT: Gas = Gas::from_tgas(5);

impl InternalFTData {
    /// Attempt to transfer FTs to a given address (will cover registration automatically).
    /// If the transfer fails, the FTs will be returned to the available balance
    /// Should *only* be invoked if the available balance is greater than or equal to the transfer amount.
    pub fn claim_ft_asset(
        &mut self,
        receiver_id: &AccountId,
        transfer_amount: &Balance,
    ) -> Option<Promise> {
        if !self.enough_balance(&transfer_amount) {
            near_sdk::log!(
                "not enough balance to transfer. Found {} but needed {}. Skipping asset claim.",
                self.balance_avail,
                transfer_amount
            );
            return None;
        }

        // Decrement the available balance and then invoke the transfer
        self.balance_avail -= transfer_amount;

        // Create a new batch promise to pay storage and transfer FTs to the new account ID
        let mut batch_transfer = Promise::new(self.contract_id.clone());

        // Pay the required storage as outlined in the AccountData. This will run first and then we send the fungible tokens
        // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
        batch_transfer = batch_transfer.function_call_weight(
            "storage_deposit".to_string(),
            json!({ "account_id": receiver_id }).to_string().into(),
            NearToken::from_near(self.registration_cost),
            MIN_GAS_FOR_STORAGE_DEPOSIT,
            GasWeight(0),
        );

        // Send the fungible tokens (after the storage attached_deposit is finished since these run sequentially)
        // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
        batch_transfer = batch_transfer.function_call_weight(
            "ft_transfer".to_string(),
            json!({ "receiver_id": receiver_id, "amount": transfer_amount.to_string(), "memo": "Keypom FT Tokens" }).to_string().into(),
            NearToken::from_near(1),
            MIN_GAS_FOR_FT_TRANSFER,
            GasWeight(0)
        );

        Some(batch_transfer)
    }
}

