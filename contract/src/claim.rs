use crate::*;

#[near_bindgen]
impl LinkDropProxy {
    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of regular claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return account data.
        let (_, funder_id, balance, storage_used, cb_data_sent, token_sender, token_contract, nft_id, ft_balance, ft_storage) = self.process_claim();
        
        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("in regular claim right before transfer: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Send the existing account ID the desired linkdrop balance.
        Promise::new(account_id.clone()).transfer(balance.0)
        .then(ext_self::on_claim(
            account_id,
            funder_id,
            balance,
            storage_used,
            cb_data_sent,
            token_sender,
            token_contract,
            nft_id, 
            ft_balance,
            ft_storage,
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
        let (_, funder_id, balance, storage_used, cb_data_sent, token_sender, token_contract, nft_id, ft_balance, ft_storage) = self.process_claim();

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
            funder_id,
            balance,
            storage_used,
            cb_data_sent,
            token_sender,
            token_contract,
            nft_id, 
            ft_balance,
            ft_storage,
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
        account_id: AccountId, 
        funder_id: AccountId, 
        balance: U128, 
        storage_used: U128,
        cb_data_sent: bool,
        token_sender: Option<AccountId>,
        token_contract: Option<AccountId>,
        nft_id: Option<String>, 
        ft_balance: Option<U128>,
        ft_storage: Option<U128>
    ) -> bool {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("CB Data Sent: {}",cb_data_sent));
        env::log_str(&format!("Beginning of on claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
        assert_eq!(env::promise_results_count(), 1, "no promise result");
        let claim_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + storage_used.0 - BURNT_GAS;
        
        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            amount_to_refund += balance.0
        }

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("In on claim before refund used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Refunding
        Promise::new(funder_id).transfer(amount_to_refund);

        /*
            Non Fungible Tokens
        */
        if nft_id.is_some() && cb_data_sent == true {
            let nft_contract_id = token_contract.clone().expect("no contract ID found");
            let token_id = nft_id.expect("no token Id found");

            used_gas = env::used_gas();
            prepaid_gas = env::prepaid_gas();

            env::log_str(&format!("In on claim before nft transfer used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

            // Only send the NFT to the new account if the claim was successful. We return the NFT if it wasn't successful in the else case.
            if claim_succeeded {
                // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
                ext_nft_contract::nft_transfer(
                    account_id.clone(), 
                    token_id.clone(),
                    None,
                    Some("Linkdropped NFT".to_string()),
                    nft_contract_id.clone(),
                    1,
                    GAS_FOR_SIMPLE_NFT_TRANSFER,
                ).then(ext_self::nft_resolve_transfer(
                    token_id,
                    token_sender.clone().expect("no token sender associated with NFT"),
                    nft_contract_id,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_RESOLVE_TRANSFER,
                ));
            } else {
                // CCC to the NFT contract to transfer the token to the original token sender. No callback necessary.
                ext_nft_contract::nft_transfer(
                    token_sender.clone().expect("no token sender associated with NFT"), 
                    token_id,
                    None,
                    Some("Linkdropped NFT".to_string()),
                    nft_contract_id,
                    1,
                    GAS_FOR_SIMPLE_NFT_TRANSFER,
                );
            }
            
        }

        /*
            Fungible Tokens
        */
        if ft_balance.is_some() && cb_data_sent == true {
            let ft_contract_id = token_contract.expect("no contract ID found");
            let amount = ft_balance.expect("no ft balance found");
            let storage_required = ft_storage.expect("no ft storage found");

            used_gas = env::used_gas();
            prepaid_gas = env::prepaid_gas();

            env::log_str(&format!("In on claim before ft transfer used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

            // Only send the fungible tokens to the new account if the claim was successful. We return the FTs if it wasn't successful in the else case.
            if claim_succeeded {
                // Create a new batch promise to pay storage and transfer NFTs to the new account ID
                let batch_ft_promise_id = env::promise_batch_create(&ft_contract_id);

                // Pay the required storage as outlined in the AccountData. This will run first and then we send the fungible tokens
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "storage_deposit",
                    json!({ "account_id": account_id }).to_string().as_bytes(),
                    storage_required.0,
                    GAS_FOR_STORAGE_DEPOSIT
                );

                // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "ft_transfer",
                    json!({ "receiver_id": account_id, "amount": amount, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                    1,
                    GAS_FOR_FT_TRANSFER
                );

                // Callback after both the storage was deposited and the fungible tokens were sent
                env::promise_then(
                    batch_ft_promise_id,
                    env::current_account_id(),
                    "ft_resolve_batch",
                    json!({ "amount": amount, "token_sender": token_sender, "token_contract": ft_contract_id }).to_string().as_bytes(),
                    NO_DEPOSIT,
                    GAS_FOR_RESOLVE_BATCH
                );
            } else {
                // Create a new batch promise to pay storage and refund the FTs to the original sender 
                let batch_ft_promise_id = env::promise_batch_create(&ft_contract_id);

                // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "storage_deposit",
                    json!({ "account_id": token_sender }).to_string().as_bytes(),
                    amount.0,
                    GAS_FOR_STORAGE_DEPOSIT
                );

                // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "ft_transfer",
                    json!({ "receiver_id": token_sender, "amount": amount, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                    1,
                    GAS_FOR_FT_TRANSFER
                );

                // Return the result of the batch as the return of the function
                env::promise_return(batch_ft_promise_id);
            }
            
        }

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of on claim function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        claim_succeeded
    }

    /// Internal method for deleting the used key and removing / returning account data.
    fn process_claim(&mut self) -> (PublicKey, AccountId, U128, U128, bool, Option<AccountId>, Option<AccountId>, Option<String>, Option<U128>, Option<U128>) {
        // Ensure only the current contract is calling the method using the access key
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );

        // Get the PK of the signer which should be the contract's function call access key
        let signer_pk = env::signer_account_pk();

        // By default, every key should have account data
        let account_data = self.accounts
            .remove(&signer_pk)
            .expect("Key missing");

        let funder_id = account_data.funder_id;
        let balance = account_data.balance;
        let cb_data_sent = account_data.cb_data_sent;
        let storage_used = account_data.storage_used;
        let mut token_sender = None;
        let mut token_contract = None;
        let mut nft_id = None;
        let mut ft_balance = None;
        let mut ft_storage = None;

        // If the linkdrop has a callback ID, return the specific callback info. Otherwise, return only account data. 
        if let Some(nonce) = account_data.cb_id {
            let cb_type = account_data.cb_type.unwrap();

            // Check for the specific callback type and return the info.
            match cb_type {
                CBType::NFT => {
                    let nft_data = self.nft.remove(&nonce).expect("No NFT data found for the public key");
                    token_sender = Some(nft_data.nft_sender);
                    token_contract = Some(nft_data.nft_contract);
                    nft_id = Some(nft_data.nft_token_id);
                },
                CBType::FT => {
                    let ft_data = self.ft.remove(&nonce).expect("No FT data found for the public key");
                    token_sender = Some(ft_data.ft_sender);
                    token_contract = Some(ft_data.ft_contract);
                    ft_balance = Some(ft_data.ft_balance);
                    ft_storage = Some(ft_data.ft_storage.expect("FT storage missing"));
                },
                CBType::FC => {
                }
            }
        }

        // Delete the key
        Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());

        // Return account data info
        (signer_pk, funder_id, balance, storage_used, cb_data_sent, token_sender, token_contract, nft_id, ft_balance, ft_storage)
    }
}