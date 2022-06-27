use crate::*;

/// Keep track of nft data 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct NFTData {
    pub nft_sender: AccountId,
    pub nft_contract: AccountId,
    pub nft_token_id: String,
}

#[near_bindgen]
impl DropZone {
    pub fn nft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        msg: U128,
    ) -> PromiseOrValue<bool> {
        require!(token_id.len() <= 256, "Contract cannot accept token IDs of length greater than 256 bytes");

        let contract_id = env::predecessor_account_id();

        let mut drop = self.drop_for_id.get(&msg.0).expect("No drop found for ID");
        let NFTData { nft_sender, nft_contract, nft_token_id } = drop.nft_data.as_ref().expect("No NFT data found for drop");

        require!(nft_sender == &sender_id && nft_contract == &contract_id && nft_token_id == &token_id, "NFT data must match what was sent");
        
        drop.keys_registered += 1;

        // Ensure that the number of keys registered cannot exceed the drop length
        if drop.keys_registered > drop.pks.len() {
            drop.keys_registered = drop.pks.len()
        }

        // Insert the drop with the updated data
        self.drop_for_id.insert(&msg.0, &drop);

        // Everything went well and we don't need to return the token.
        PromiseOrValue::Value(false)
    }

    /// self callback checks if NFT was successfully transferred to the new account. If yes, do nothing. If no, refund original sender
    pub fn nft_resolve_transfer(
        &mut self, 
        token_id: String, 
        token_sender: AccountId,
        token_contract: AccountId 
    ) -> bool {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of resolve transfer used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
        assert_eq!(env::promise_results_count(), 1, "no promise result");
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));
        

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();
        env::log_str(&format!("Before refunding token sender in resolve transfer: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !transfer_succeeded {
            env::log_str("Attempt to transfer the new account was unsuccessful. Sending the NFT to the original sender.");
            ext_nft_contract::ext(token_contract)
                // Call nft transfer with the min GAS and 1 yoctoNEAR. all unspent GAS will be added on top
                .with_static_gas(MIN_GAS_FOR_SIMPLE_NFT_TRANSFER)
                .with_attached_deposit(1)
                .nft_transfer(
                    token_sender, 
                    token_id,
                    None,
                    Some("Linkdropped NFT Refund".to_string()),
                );
        }

        transfer_succeeded
    }
}
