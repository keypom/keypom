use crate::*;

#[near_bindgen]
impl LinkDropProxy {
    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        // Ensure the user attaches enough GAS and isn't doing anything malicious / third party.
        assert!(prepaid_gas >= MIN_PREPAID_GAS_FOR_CLAIM, "Cannot attach less than the minimum amount of prepaid gas");

        env::log_str(&format!("Beginning of regular claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return account data, and optionally callback data.
        let (
            account_data,
            ft_data_option,
            nft_data_option,
            fc_data_option
        ) = self.process_claim();
        
        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("in regular claim right before transfer: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Send the existing account ID the desired linkdrop balance.
        Promise::new(account_id.clone()).transfer(account_data.balance.0)
        .then(
            if let Some(ft_data) = ft_data_option {
                ext_self::on_claim_ft(
                    // Account ID that claimed the linkdrop
                    account_id, 
                    // Account ID that funded the linkdrop
                    account_data.funder_id, 
                    // Balance associated with the linkdrop
                    account_data.balance, 
                    // How much storage was used to store linkdrop info
                    account_data.storage_used,
                    // Did the sender end up sending the FTs to the contract
                    account_data.cb_data_sent,
                    // Who sent the FTs?
                    ft_data.ft_sender,
                    // Where are the FTs stored
                    ft_data.ft_contract,
                    // How many FTs should we send
                    ft_data.ft_balance,
                    // How much storage does it cost to register the new account
                    ft_data.ft_storage.unwrap(),
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_ON_CLAIM,
                )
            } else if let Some(nft_data) = nft_data_option {
                ext_self::on_claim_nft(
                    // Account ID that claimed the linkdrop
                    account_id, 
                    // Account ID that funded the linkdrop
                    account_data.funder_id, 
                    // Balance associated with the linkdrop
                    account_data.balance, 
                    // How much storage was used to store linkdrop info
                    account_data.storage_used,
                    // Did the sender end up sending the FTs to the contract
                    account_data.cb_data_sent,
                    // Sender of the NFT
                    nft_data.nft_sender,
                    // Contract where the NFT is stored
                    nft_data.nft_contract,
                    // Token ID for the NFT
                    nft_data.nft_token_id,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_ON_CLAIM,
                )
            } else if let Some(fc_data) = fc_data_option {
                ext_self::on_claim_fc(
                    // Account ID that claimed the linkdrop
                    account_id, 
                    // Account ID that funded the linkdrop
                    account_data.funder_id, 
                    // Balance associated with the linkdrop
                    account_data.balance, 
                    // How much storage was used to store linkdrop info
                    account_data.storage_used,
                    // Receiver of the function call
                    fc_data.receiver,
                    // Method to call on the contract
                    fc_data.method,
                    // What args to pass in
                    fc_data.args,
                    // What deposit should we attach
                    fc_data.deposit,
                    // Should the refund be sent to the funder or attached to the deposit
                    fc_data.refund_to_deposit,
                    // Should we add the account ID as part of the args and what key should it live in
                    fc_data.claimed_account_field,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_ON_CLAIM,
                )
            } else {
                ext_self::on_claim_simple(
                    // Account ID that funded the linkdrop
                    account_data.funder_id, 
                    // Balance associated with the linkdrop
                    account_data.balance, 
                    // How much storage was used to store linkdrop info
                    account_data.storage_used,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_ON_CLAIM,
                )
            }
        );

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

        // Ensure the user attaches enough GAS and isn't doing anything malicious / third party.
        assert!(prepaid_gas >= MIN_PREPAID_GAS_FOR_CLAIM, "Cannot attach less than the minimum amount of prepaid gas");

        env::log_str(&format!("Beginning of CAAC used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return account data, and optionally callback data.
        let (
            account_data,
            ft_data_option,
            nft_data_option,
            fc_data_option
        ) = self.process_claim();

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("In CAAC after process claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));
        
        // CCC to the linkdrop contract to create the account with the desired balance as the linkdrop amount
        ext_linkdrop::create_account(
            new_account_id.clone(),
            new_public_key,
            self.linkdrop_contract.clone(),
            account_data.balance.0,
            GAS_FOR_CREATE_ACCOUNT,
        ).then(
            if let Some(ft_data) = ft_data_option {
                ext_self::on_claim_ft(
                    // Account ID that claimed the linkdrop
                    new_account_id, 
                    // Account ID that funded the linkdrop
                    account_data.funder_id, 
                    // Balance associated with the linkdrop
                    account_data.balance, 
                    // How much storage was used to store linkdrop info
                    account_data.storage_used,
                    // Did the sender end up sending the FTs to the contract
                    account_data.cb_data_sent,
                    // Who sent the FTs?
                    ft_data.ft_sender,
                    // Where are the FTs stored
                    ft_data.ft_contract,
                    // How many FTs should we send
                    ft_data.ft_balance,
                    // How much storage does it cost to register the new account
                    ft_data.ft_storage.unwrap(),
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_ON_CLAIM,
                )
            } else if let Some(nft_data) = nft_data_option {
                ext_self::on_claim_nft(
                    // Account ID that claimed the linkdrop
                    new_account_id, 
                    // Account ID that funded the linkdrop
                    account_data.funder_id, 
                    // Balance associated with the linkdrop
                    account_data.balance, 
                    // How much storage was used to store linkdrop info
                    account_data.storage_used,
                    // Did the sender end up sending the FTs to the contract
                    account_data.cb_data_sent,
                    // Sender of the NFT
                    nft_data.nft_sender,
                    // Contract where the NFT is stored
                    nft_data.nft_contract,
                    // Token ID for the NFT
                    nft_data.nft_token_id,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_ON_CLAIM,
                )
            } else if let Some(fc_data) = fc_data_option {
                ext_self::on_claim_fc(
                    // Account ID that claimed the linkdrop
                    new_account_id, 
                    // Account ID that funded the linkdrop
                    account_data.funder_id, 
                    // Balance associated with the linkdrop
                    account_data.balance, 
                    // How much storage was used to store linkdrop info
                    account_data.storage_used,
                    // Receiver of the function call
                    fc_data.receiver,
                    // Method to call on the contract
                    fc_data.method,
                    // What args to pass in
                    fc_data.args,
                    // What deposit should we attach
                    fc_data.deposit,
                    // Should the refund be sent to the funder or attached to the deposit
                    fc_data.refund_to_deposit,
                    // Should we add the account ID as part of the args and what key should it live in
                    fc_data.claimed_account_field,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_ON_CLAIM,
                )
            } else {
                ext_self::on_claim_simple(
                    // Account ID that funded the linkdrop
                    account_data.funder_id, 
                    // Balance associated with the linkdrop
                    account_data.balance, 
                    // How much storage was used to store linkdrop info
                    account_data.storage_used,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_ON_CLAIM,
                )
            }
        );

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of on CAAC function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

    }

    /// Internal method for deleting the used key and removing / returning linkdrop data.
    fn process_claim(&mut self) -> (
        AccountData,
        Option<FTData>,
        Option<NFTData>,
        Option<FCData>,
    ) {
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

        // Default all callback data to None
        let mut ft_data = None;
        let mut nft_data = None;
        let mut fc_data = None;

        // Delete the key
        Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());

        // If there's a Nonce, remove all occurrences of the nonce and return the linkdrop data
        if let Some(nonce) = account_data.cb_id {
            ft_data = self.ft.remove(&nonce);
            nft_data = self.nft.remove(&nonce);
            fc_data = self.fc.remove(&nonce);
        }

        (
            account_data,
            ft_data,
            nft_data,
            fc_data
        )
    }

    /// self callback for simple linkdrops with no FTs, NFTs, or FCs.
    pub fn on_claim_simple(
        &mut self,
        // Account ID that sent the funds for the linkdrop
        funder_id: AccountId, 
        // Balance contained within the linkdrop
        balance: U128, 
        // How much storage was used up for the linkdrop
        storage_used: U128,
    ) -> bool {        
        // Get the status of the cross contract call
        let claim_succeeded = self.assert_success(); 

        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Simple on claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + storage_used.0 - BURNT_GAS;
        
        env::log_str(&format!("Refund Amount: {}, Access Key Allowance: {}, Access Key Storage: {}, Storage Used: {}, Burnt GAS: {}", yocto_to_near(amount_to_refund), yocto_to_near(ACCESS_KEY_ALLOWANCE), yocto_to_near(ACCESS_KEY_STORAGE), yocto_to_near(storage_used.0), yocto_to_near(BURNT_GAS)));

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance as well: {}", balance.0));
            amount_to_refund += balance.0
        }

        env::log_str(&format!("Refunding funder: {:?} For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
        // Send the necessary funds to the funder
        Promise::new(funder_id.clone()).transfer(amount_to_refund);

        claim_succeeded
    }

    /// self callback for FT linkdrop
    pub fn on_claim_ft(
        &mut self, 
        // Account ID that claimed the linkdrop
        account_id: AccountId, 
        // Account ID that funded the linkdrop
        funder_id: AccountId, 
        // Balance associated with the linkdrop
        balance: U128, 
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Did the sender end up sending the FTs to the contract
        did_send_fts: bool,
        // Who sent the FTs?
        ft_sender: AccountId,
        // Where are the FTs stored
        ft_contract: AccountId,
        // How many FTs should we send
        ft_balance: U128,
        // How much storage does it cost to register the new account
        ft_storage: U128,
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Did FTs get sent: {}",did_send_fts));
        env::log_str(&format!("Beginning of on claim FT used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

       // Get the status of the cross contract call
       let claim_succeeded = self.assert_success(); 

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + storage_used.0 - BURNT_GAS;
        
        env::log_str(&format!("Refund Amount: {}, Access Key Allowance: {}, Access Key Storage: {}, Storage Used: {}, Burnt GAS: {}", yocto_to_near(amount_to_refund), yocto_to_near(ACCESS_KEY_ALLOWANCE), yocto_to_near(ACCESS_KEY_STORAGE), yocto_to_near(storage_used.0), yocto_to_near(BURNT_GAS)));

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance as well: {}", balance.0));
            amount_to_refund += balance.0
        }

        env::log_str(&format!("Refunding funder: {:?} For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
        // Perform the refund for the necessary amount
        Promise::new(funder_id.clone()).transfer(amount_to_refund);

        /*
            Fungible Tokens. 
            - Only send the FTs if the sender ended up sending the contract the tokens.
        */
        if did_send_fts == true {
            // Only send the fungible tokens to the new account if the claim was successful. We return the FTs if it wasn't successful in the else case.
            if claim_succeeded {
                // Create a new batch promise to pay storage and transfer NFTs to the new account ID
                let batch_ft_promise_id = env::promise_batch_create(&ft_contract);

                // Pay the required storage as outlined in the AccountData. This will run first and then we send the fungible tokens
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "storage_deposit",
                    json!({ "account_id": account_id }).to_string().as_bytes(),
                    ft_storage.0,
                    GAS_FOR_STORAGE_DEPOSIT
                );

                // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "ft_transfer",
                    json!({ "receiver_id": account_id, "amount": ft_balance, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                    1,
                    GAS_FOR_FT_TRANSFER
                );

                // Callback after both the storage was deposited and the fungible tokens were sent
                env::promise_then(
                    batch_ft_promise_id,
                    env::current_account_id(),
                    "ft_resolve_batch",
                    json!({ "amount": ft_balance, "token_sender": ft_sender, "token_contract": ft_contract }).to_string().as_bytes(),
                    NO_DEPOSIT,
                    GAS_FOR_RESOLVE_BATCH
                );
            } else {
                // Create a new batch promise to pay storage and refund the FTs to the original sender 
                let batch_ft_promise_id = env::promise_batch_create(&ft_contract);

                // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "storage_deposit",
                    json!({ "account_id": ft_sender }).to_string().as_bytes(),
                    ft_storage.0,
                    GAS_FOR_STORAGE_DEPOSIT
                );

                // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "ft_transfer",
                    json!({ "receiver_id": ft_sender, "amount": ft_balance, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                    1,
                    GAS_FOR_FT_TRANSFER
                );

                // Return the result of the batch as the return of the function
                env::promise_return(batch_ft_promise_id);
            }
        } else {
            env::log_str("Cannot send FTs since the sender never transferred the contract the tokens.");
        }

        claim_succeeded
    }

    /// self callback for a linkdrop loaded with an NFT
    pub fn on_claim_nft(&mut self, 
        // Account ID that claimed the linkdrop
        account_id: AccountId, 
        // Account ID that funded the linkdrop
        funder_id: AccountId, 
        // Balance associated with the linkdrop
        balance: U128, 
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Did the sender end up sending the NFT to the contract
        did_send_nft: bool,
        // Sender of the NFT
        nft_sender: AccountId,
        // Contract where the NFT is stored
        nft_contract: AccountId,
        // Token ID for the NFT
        token_id: String, 
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Was NFT sent to contract: {}",did_send_nft));
        env::log_str(&format!("Beginning of on claim NFT used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Get the status of the cross contract call
        let claim_succeeded = self.assert_success(); 

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + storage_used.0 - BURNT_GAS;
        
        env::log_str(&format!("Refund Amount: {}, Access Key Allowance: {}, Access Key Storage: {}, Storage Used: {}, Burnt GAS: {}", yocto_to_near(amount_to_refund), yocto_to_near(ACCESS_KEY_ALLOWANCE), yocto_to_near(ACCESS_KEY_STORAGE), yocto_to_near(storage_used.0), yocto_to_near(BURNT_GAS)));

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance as well: {}", balance.0));
            amount_to_refund += balance.0
        }

        env::log_str(&format!("Refunding funder: {:?} For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
        // Perform the refund for the necessary amount
        Promise::new(funder_id.clone()).transfer(amount_to_refund);

        /*
            Non Fungible Tokens
        */
        if did_send_nft == true {
            // Only send the NFT to the new account if the claim was successful. We return the NFT if it wasn't successful in the else case.
            if claim_succeeded {
                // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
                ext_nft_contract::nft_transfer(
                    account_id.clone(), 
                    token_id.clone(),
                    None,
                    Some("Linkdropped NFT".to_string()),
                    nft_contract.clone(),
                    1,
                    GAS_FOR_SIMPLE_NFT_TRANSFER,
                ).then(ext_self::nft_resolve_transfer(
                    token_id,
                    nft_sender,
                    nft_contract,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_RESOLVE_TRANSFER,
                ));
            } else {
                // CCC to the NFT contract to transfer the token to the original token sender. No callback necessary.
                ext_nft_contract::nft_transfer(
                    nft_sender, 
                    token_id,
                    None,
                    Some("Linkdropped NFT".to_string()),
                    nft_contract,
                    1,
                    GAS_FOR_SIMPLE_NFT_TRANSFER,
                );
            }
            
        } else {
            env::log_str("Cannot send FTs since the sender never transferred the contract the tokens.");
        }

        claim_succeeded
    }

    /// self callback checks if account was created successfully or not. If yes, refunds excess storage, sends NFTs, FTs etc..
    pub fn on_claim_fc(&mut self, 
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        funder_id: AccountId, 
        // Balance associated with the linkdrop
        balance: U128, 
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Receiver of the function call
        receiver: AccountId,
        // Method to call on the contract
        method: String,
        // What args to pass in
        args: String,
        // What deposit should we attach
        deposit: U128,
        // Should the refund be sent to the funder or attached to the deposit
        add_refund_to_deposit: Option<bool>,
        // Should we add the account ID as part of the args and what key should it live in
        claimed_account_field: Option<String>,
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of on claim Function Call used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Get the status of the cross contract call
        let claim_succeeded = self.assert_success(); 

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + storage_used.0 - BURNT_GAS;
        
        env::log_str(&format!("Refund Amount: {}, Access Key Allowance: {}, Access Key Storage: {}, Storage Used: {}, Burnt GAS: {}", yocto_to_near(amount_to_refund), yocto_to_near(ACCESS_KEY_ALLOWANCE), yocto_to_near(ACCESS_KEY_STORAGE), yocto_to_near(storage_used.0), yocto_to_near(BURNT_GAS)));

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance as well: {}", balance.0));
            amount_to_refund += balance.0
        }

        /* 
            If the claim is not successful, we should always refund. The only case where we refund
            if the claim was successful is if the user specified that the refund should go into the
            deposit.

            0 0     Refund     !success  -> do refund
            0 1     Refund      success  -> do refund
            1 0     No Refund  !success  -> do refund
            1 1     No Refund   Success  -> don't do refund
        */ 
        if !claim_succeeded || (!add_refund_to_deposit.unwrap_or(false) && claim_succeeded) {
            // Refunding
            env::log_str(&format!("Refunding funder: {:?} For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
            Promise::new(funder_id.clone()).transfer(amount_to_refund);
        } else {
            env::log_str(&format!("Skipping the refund to funder: {:?} claim success: {:?} refund to deposit?: {:?}", funder_id, claim_succeeded, add_refund_to_deposit.unwrap_or(false)));
        }

        /*
            Function Calls
        */
        // Only call the function if the claim was successful. If not, refund the callback sender for the callback deposit. 
        if claim_succeeded {
            let mut final_args = args.clone();

            // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
            if let Some(account_field) = claimed_account_field {
                final_args.insert_str(final_args.len()-1, &format!(",\"{}\":\"{}\"", account_field, account_id));
                env::log_str(&format!("Adding claimed account ID to specified field: {:?} in args: {:?}", account_field, args));
            }
        
            env::log_str(&format!("Attaching Total: {:?} Deposit: {:?} Should Refund?: {:?} Amount To Refund: {:?} With args: {:?}", yocto_to_near(deposit.0 + if add_refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}), yocto_to_near(deposit.0), add_refund_to_deposit.unwrap_or(false), yocto_to_near(amount_to_refund), final_args));

            Promise::new(receiver).function_call(
                method, 
                final_args.as_bytes().to_vec(), 
                // The claim is successful so attach the amount to refund to the deposit instead of refunding the funder.
                deposit.0 + if add_refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}, 
                GAS_FOR_CALLBACK_FUNCTION_CALL
            );

        } else {
            env::log_str(&format!("Claim unsuccessful. Refunding callback deposit as well: {}", yocto_to_near(deposit.0)));
            // Refunding
            Promise::new(funder_id).transfer(deposit.0);
        }

        claim_succeeded
    }
}