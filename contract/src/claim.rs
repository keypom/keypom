use crate::*;

#[near_bindgen]
impl LinkDropProxy {
    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of regular claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return account data.
        let (_, balance, funder_id, nft_id, token_contract, token_sender) = self.process_claim();
        
        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("in regular claim right before transfer: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Send the existing account ID the desired linkdrop balance.
        Promise::new(account_id.clone()).transfer(balance.0)
        .then(ext_self::on_claim(
            account_id,
            balance,
            funder_id,
            nft_id, 
            token_contract,
            token_sender,
            env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_ON_CLAIM,
        ));

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of regular claim function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

    }

    /// Create new account and and claim tokens to it.
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: AccountId,
        new_public_key: PublicKey,
    ) {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of CAAC used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return account data.
        let (_, balance, funder_id, nft_id, token_contract, token_sender) = self.process_claim();

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("In CAAC after process claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));
        
        // CCC to the linkdrop contract to create the account with the desired balance as the linkdrop amount
        ext_linkdrop::create_account(
            new_account_id.clone(),
            new_public_key,
            self.linkdrop_contract.clone(),
            balance.0,
            GAS_FOR_CREATE_ACCOUNT,
        ).then(ext_self::on_claim(
            new_account_id,
            balance,
            funder_id,
            nft_id, 
            token_contract,
            token_sender,
            env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_ON_CLAIM,
        ));

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of on CAAC function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

    }

    /// self callback checks if account was created successfully or not. If yes, refunds excess storage, sends NFTs, FTs etc..
    pub fn on_claim(&mut self, 
        new_account_id: AccountId, 
        balance: U128, 
        funder_id: AccountId, 
        nft_id: Option<String>, 
        token_contract: Option<AccountId>,
        token_sender: Option<AccountId>
    ) -> bool {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of on claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
        assert_eq!(env::promise_results_count(), 1, "no promise result");
        let claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE - BURNT_GAS;
        
        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            amount_to_refund += balance.0
        }

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("In on claim before refund used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        Promise::new(funder_id).transfer(amount_to_refund);

        // If NFT isn't successfully transferred, should send it back to it's original owner? 
        if nft_id.is_some() {
            let nft_contract_id = token_contract.expect("no contract ID found");
            let token_id = nft_id.expect("no token Id found");

            used_gas = env::used_gas();
            prepaid_gas = env::prepaid_gas();

            env::log_str(&format!("In on claim before nft transfer used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

            // Only send the NFT to the new account if the claim was successful. We return the NFT if it wasn't successful in the else case.
            if claim_succeeded {
                // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
                ext_nft_contract::nft_transfer(
                    new_account_id, 
                    token_id.clone(),
                    None,
                    Some("Linkdropped NFT".to_string()),
                    nft_contract_id.clone(),
                    1,
                    GAS_FOR_COMPLEX_NFT_TRANSFER,
                ).then(ext_self::nft_resolve_transfer(
                    token_id,
                    token_sender.expect("no token sender associated with NFT"),
                    nft_contract_id,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_RESOLVE_TRANSFER,
                ));
            } else {
                // CCC to the NFT contract to transfer the token to the original token sender. No callback necessary.
                ext_nft_contract::nft_transfer(
                    token_sender.expect("no token sender associated with NFT"), 
                    token_id,
                    None,
                    Some("Linkdropped NFT".to_string()),
                    nft_contract_id,
                    1,
                    GAS_FOR_SIMPLE_NFT_TRANSFER,
                );
            }
            
        }

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of on claim function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        claim_succeeded
    }

    /// Internal method for deleting the used key and removing / returning account data.
    fn process_claim(&mut self) -> (PublicKey, U128, AccountId, Option<String>, Option<AccountId>, Option<AccountId>) {
        // Ensure only the current contract is calling the method using the access key
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );

        // Get the PK of the signer which should be the contract's function call access key
        let signer_pk = env::signer_account_pk();
        let AccountData {
            funder_id,
            balance,
            token_contract,
            nft_id,
            token_sender,
        } = self.accounts
            .remove(&signer_pk)
            .expect("Missing public key");

        // Delete the key
        Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());

        // Return account data info
        (signer_pk, balance, funder_id, nft_id, token_contract, token_sender)
    }
}