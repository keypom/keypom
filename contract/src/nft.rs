use crate::*;

#[near_bindgen]
impl LinkDropProxy {
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
            funder_id: _,
            balance,
            token_contract,
            nft_id: _,
            token_sender: _
        } = self.accounts
            .get(&msg)
            .expect("Missing public key");

        // Ensure that both the token contract is none since we only store one set of NFT / FT data.
        assert!(token_contract.is_none(), "PK must have no external token contract.");

        //insert the NFT token ID and token contract back into the map
        self.accounts.insert(
            &msg,
            &AccountData{
                funder_id: env::predecessor_account_id(),
                balance: balance,
                nft_id: Some(token_id),
                token_contract: Some(contract_id),
                token_sender: Some(sender_id)
            },
        );

        // Everything went well and we don't need to return the token.
        PromiseOrValue::Value(false)
    }

    /// self callback checks if account was created successfully or not. If yes, refunds excess storage, sends NFTs, FTs etc..
    pub fn nft_resolve_transfer(&mut self, 
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
            ext_nft_contract::nft_transfer(
                token_sender, 
                token_id,
                None,
                Some("Linkdropped NFT Refund".to_string()),
                token_contract,
                1,
                GAS_FOR_SIMPLE_NFT_TRANSFER,
            );
        }

        transfer_succeeded
    }
}
