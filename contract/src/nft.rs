use crate::*;

/// Keep track of nft data 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
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
        msg: PublicKey,
    ) -> PromiseOrValue<bool> {
        assert!(token_id.len() <= 256, "Contract cannot accept token IDs of length greater than 256 bytes");

        let contract_id = env::predecessor_account_id();

        // No need to assert that the funder is the sender since we don't wanna enforce anything unnecessary.
        // All that matters is we've received the token and that the token belongs to some public key.
        let AccountData {
            funder_id,
            balance,
            storage_used,
            cb_id,
            cb_data_sent,
        } = self.data_for_pk
            .get(&msg)
            .expect("Missing public key");

        // Ensure there's a callback ID (meaning the linkdrop is not a regular linkdrop)
        let callback_id = cb_id.expect("Callback ID must be set");
        
        // Assert that the FTs have NOT been sent yet
        assert!(cb_data_sent == false, "NFT already sent. Cannot send more.");

        // Ensure that the linkdrop contains FT data already
        let NFTData { 
            nft_sender,
            nft_contract,
            nft_token_id
        } = self.nft.get(&callback_id).expect("No NFT data found for the unique callback ID.");

        assert!(nft_sender == sender_id && nft_contract == contract_id && nft_token_id == token_id, "NFT data must match what was sent");
        

        // Insert the account data back with the cb data sent set to true
        self.data_for_pk.insert(
            &msg,
            &AccountData{
                funder_id,
                balance,
                storage_used,
                cb_id,
                cb_data_sent: true,
            },
        );

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
