use near_sdk::{Gas, GasWeight, serde_json::json};

use crate::*;

/// Gas needed to execute any logic in the nft claim function
/// 2 TGas + 1 * CCC gas (since there is 1 CCCs)
/// 7 TGas
pub const GAS_FOR_NFT_CLAIM_LOGIC: Gas = Gas(2_000_000_000_000 + MIN_BASE_GAS_FOR_ONE_CCC.0);
/// Minimum Gas required to perform a simple NFT transfer.
/// 5 TGas
pub const MIN_GAS_FOR_NFT_TRANSFER: Gas = Gas(5_000_000_000_000);

impl InternalNFTData {
    /// Attempt to transfer FTs to a given address (will cover registration automatically).
    /// If the transfer fails, the FTs will be returned to the available balance
    /// Should *only* be invoked if the available balance is greater than or equal to the transfer amount.
    pub fn claim_nft_asset(&mut self, receiver_id: &AccountId) -> Promise {
        if !self.enough_tokens() {
            near_sdk::log!("No NFTs available to transfer. Skipping asset claim.");
            return Promise::new(env::current_account_id());
        }
        
        // Pop the last NFT from the available NFTs
        let token_to_transfer = self.token_ids.pop().unwrap();

        // Create a new promise to transfer the NFT to the new account ID
        let transfer_promise = Promise::new(self.contract_id.clone())
            .function_call_weight(
                "nft_transfer".to_string(),
                json!({ "receiver_id": receiver_id, "token_id": token_to_transfer, "memo": "Keypom Linkdrop" }).to_string().into(),
                1,
                MIN_GAS_FOR_NFT_TRANSFER,
                GasWeight(1),
            );

        transfer_promise
    }
}