use crate::*;

/// Gas needed to execute any logic in the nft claim function
/// 2 TGas + 1 * CCC gas (since there is 1 CCCs)
/// 7 TGas
pub const GAS_FOR_NFT_CLAIM_LOGIC: Gas =
    Gas::from_gas(Gas::from_tgas(2).as_gas() + MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP.as_gas());
/// Minimum Gas required to perform a simple NFT transfer.
/// 4 TGas
pub const MIN_GAS_FOR_NFT_TRANSFER: Gas = Gas::from_tgas(4);
/// Minimum Gas required to execute any callback logic after the claim is complete
/// 2 TGas
pub const MIN_GAS_FOR_NFT_CALLBACK_LOGIC: Gas = Gas::from_tgas(2);

impl InternalNFTData {
    /// Attempt to transfer FTs to a given address (will cover registration automatically).
    /// If the transfer fails, the FTs will be returned to the available balance
    /// Should *only* be invoked if the available balance is greater than or equal to the transfer amount.
    pub fn claim_nft_asset(&mut self, receiver_id: &AccountId) -> Option<Promise> {
        if self.is_empty() {
            near_sdk::log!("No NFTs available to transfer. Skipping asset claim.");
            return None;
        }

        // Pop the last NFT from the available NFTs
        let token_to_transfer = self.token_ids.pop().unwrap();

        // Create a new promise to transfer the NFT to the new account ID
        let transfer_promise = Promise::new(self.contract_id.clone())
            .function_call_weight(
                "nft_transfer".to_string(),
                json!({ "receiver_id": receiver_id, "token_id": token_to_transfer, "memo": "Keypom Linkdrop" }).to_string().into(),
                NearToken::from_yoctonear(1),
                MIN_GAS_FOR_NFT_TRANSFER,
                GasWeight(1),
            );

        Some(transfer_promise)
    }
}

