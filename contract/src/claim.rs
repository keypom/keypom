use near_sdk::GasWeight;

use crate::*;

#[near_bindgen]
impl DropZone {
    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("Beginning of regular claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return drop data.
        let drop_data = self.process_claim();

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("in regular claim right before transfer: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Send the existing account ID the desired linkdrop balance.
        Promise::new(account_id.clone()).transfer(drop_data.balance.0)
        .then(
            if let Some(ft_data) = drop_data.ft_data {
                // Call on_claim_ft with all unspent GAS + min gas for on claim. No attached deposit.
                Self::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                .on_claim_ft(
                    // Account ID that claimed the linkdrop
                    account_id, 
                    // Account ID that funded the linkdrop
                    drop_data.funder_id, 
                    // Balance associated with the linkdrop
                    drop_data.balance, 
                    // How much storage was used to store linkdrop info
                    drop_data.storage_used_per_key,
                    // Who sent the FTs?
                    ft_data.ft_sender,
                    // Where are the FTs stored
                    ft_data.ft_contract,
                    // How many FTs should we send
                    ft_data.ft_balance,
                    // How much storage does it cost to register the new account
                    ft_data.ft_storage.unwrap(),
                )
            } else if let Some(nft_data) = drop_data.nft_data {
                // Call on_claim_nft with all unspent GAS + min gas for on claim. No attached deposit.
                Self::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                .on_claim_nft(
                    // Account ID that claimed the linkdrop
                    account_id, 
                    // Account ID that funded the linkdrop
                    drop_data.funder_id, 
                    // Balance associated with the linkdrop
                    drop_data.balance, 
                    // How much storage was used to store linkdrop info
                    drop_data.storage_used_per_key,
                    // Sender of the NFT
                    nft_data.nft_sender,
                    // Contract where the NFT is stored
                    nft_data.nft_contract,
                    // Token ID for the NFT
                    nft_data.nft_token_id,
                )
            } else if let Some(fc_data) = drop_data.fc_data {
                // Call on_claim_fc with all unspent GAS + min gas for on claim. No attached deposit.
                Self::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                .on_claim_fc(
                    // Account ID that claimed the linkdrop
                    account_id, 
                    // Account ID that funded the linkdrop
                    drop_data.funder_id, 
                    // Balance associated with the linkdrop
                    drop_data.balance, 
                    // How much storage was used to store linkdrop info
                    drop_data.storage_used_per_key,
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
                )
            } else {
                // Call on_claim_simple with all unspent GAS + min gas for on claim. No attached deposit.
                Self::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                .on_claim_simple(
                    // Account ID that funded the linkdrop
                    drop_data.funder_id, 
                    // Balance associated with the linkdrop
                    drop_data.balance, 
                    // How much storage was used to store linkdrop info
                    drop_data.storage_used_per_key,
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

        env::log_str(&format!("Beginning of CAAC used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return drop data.
        let drop_data = self.process_claim();

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("In CAAC after process claim used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));
        
        // CCC to the linkdrop contract to create the account with the desired balance as the linkdrop amount
        ext_linkdrop::ext(self.linkdrop_contract.clone())
            // Attach the balance of the linkdrop along with the exact gas for create account. No unspent GAS is attached.
            .with_attached_deposit(drop_data.balance.0)
            .with_static_gas(GAS_FOR_CREATE_ACCOUNT)
            .with_unused_gas_weight(0)
            .create_account(
                new_account_id.clone(),
                new_public_key,  
            )
        .then(
            if let Some(ft_data) = drop_data.ft_data {
                // Call on_claim_ft with all unspent GAS + min gas for on claim. No attached deposit.
                Self::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                .on_claim_ft(
                    // Account ID that claimed the linkdrop
                    new_account_id, 
                    // Account ID that funded the linkdrop
                    drop_data.funder_id, 
                    // Balance associated with the linkdrop
                    drop_data.balance, 
                    // How much storage was used to store linkdrop info
                    drop_data.storage_used_per_key,
                    // Who sent the FTs?
                    ft_data.ft_sender,
                    // Where are the FTs stored
                    ft_data.ft_contract,
                    // How many FTs should we send
                    ft_data.ft_balance,
                    // How much storage does it cost to register the new account
                    ft_data.ft_storage.unwrap(),
                )
            } else if let Some(nft_data) = drop_data.nft_data {
                // Call on_claim_nft with all unspent GAS + min gas for on claim. No attached deposit.
                Self::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                .on_claim_nft(
                    // Account ID that claimed the linkdrop
                    new_account_id, 
                    // Account ID that funded the linkdrop
                    drop_data.funder_id, 
                    // Balance associated with the linkdrop
                    drop_data.balance, 
                    // How much storage was used to store linkdrop info
                    drop_data.storage_used_per_key,
                    // Sender of the NFT
                    nft_data.nft_sender,
                    // Contract where the NFT is stored
                    nft_data.nft_contract,
                    // Token ID for the NFT
                    nft_data.nft_token_id,
                )
            } else if let Some(fc_data) = drop_data.fc_data {
                // Call on_claim_fc with all unspent GAS + min gas for on claim. No attached deposit.
                Self::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                .on_claim_fc(
                    // Account ID that claimed the linkdrop
                    new_account_id, 
                    // Account ID that funded the linkdrop
                    drop_data.funder_id, 
                    // Balance associated with the linkdrop
                    drop_data.balance, 
                    // How much storage was used to store linkdrop info
                    drop_data.storage_used_per_key,
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
                )
            } else {
                // Call on_claim_simple with all unspent GAS + min gas for on claim. No attached deposit.
                Self::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                .on_claim_simple(
                    // Account ID that funded the linkdrop
                    drop_data.funder_id,
                    // Balance associated with the linkdrop
                    drop_data.balance, 
                    // How much storage was used to store linkdrop info
                    drop_data.storage_used_per_key,
                )
            }
        );

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of on CAAC function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

    }

    /// Internal method for deleting the used key and removing / returning linkdrop data.
    fn process_claim(&mut self) -> Drop {
        // Ensure only the current contract is calling the method using the access key
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );

        // Get the PK of the signer which should be the contract's function call access key
        let signer_pk = env::signer_account_pk();

        // By default, every key should have a drop ID
        let drop_id = self.drop_id_for_pk.remove(&signer_pk).expect("No drop ID found for PK");
        env::log_str(&format!("Drop ID: {:?}", drop_id));

        // Remove the drop
        let mut drop = self.drop_for_id.remove(&drop_id).expect("drop not found");

        env::log_str(&format!("Drop PKs Len: {:?}", drop.pks.len()));
        // Remove the pk from the drop's set.
        drop.pks.remove(&signer_pk);
        env::log_str(&format!("Drop PKs Len 2: {:?}", drop.pks.len()));

        // If it's an NFT or FT drop, decrement the registered keys
        if drop.ft_data.is_some() || drop.nft_data.is_some() {
            if drop.keys_registered == 0 {
                env::panic_str("Key not registered. Assets must be sent")
            }

            drop.keys_registered -= 1;
        }
        
        // If there are keys still left in the drop, add the drop back in with updated data
        if !drop.pks.is_empty() {
            // Add drop back with the updated data.
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
        } else {
            // Remove the drop ID from the funder's list if the drop is now empty
            self.internal_remove_drop_for_funder(&drop.funder_id, &drop_id);
        }

        env::log_str(&format!("!Is Empty?: {:?}", !drop.pks.is_empty()));

        // Return the drop
        drop
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
        // Only send the fungible tokens to the new account if the claim was successful. We return the FTs if it wasn't successful in the else case.
        if claim_succeeded {
            // Create a new batch promise to pay storage and transfer NFTs to the new account ID
            let batch_ft_promise_id = env::promise_batch_create(&ft_contract);

            // Pay the required storage as outlined in the AccountData. This will run first and then we send the fungible tokens
            // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "storage_deposit",
                json!({ "account_id": account_id }).to_string().as_bytes(),
                ft_storage.0,
                MIN_GAS_FOR_STORAGE_DEPOSIT,
                GasWeight(1)
            );  

            // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "ft_transfer",
                json!({ "receiver_id": account_id, "amount": ft_balance, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                1,
                MIN_GAS_FOR_FT_TRANSFER,
                GasWeight(1)
            );

            // Create the second batch promise to execute after the batch_ft_promise_id batch is finished executing.
            // It will execute on the current account ID (this contract)
            let batch_ft_resolve_promise_id = env::promise_batch_then(batch_ft_promise_id, &env::current_account_id());

            // Execute a function call as part of the resolved promise index created in promise_batch_then
            // Callback after both the storage was deposited and the fungible tokens were sent
            // Call the function with the min GAS and then attach 3/5 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_resolve_promise_id,
                "ft_resolve_batch",
                json!({ "amount": ft_balance, "token_sender": ft_sender, "token_contract": ft_contract }).to_string().as_bytes(),
                NO_DEPOSIT,
                MIN_GAS_FOR_RESOLVE_BATCH,
                GasWeight(3)
            );

        } else {
            // Create a new batch promise to pay storage and refund the FTs to the original sender 
            let batch_ft_promise_id = env::promise_batch_create(&ft_contract);

            // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "storage_deposit",
                json!({ "account_id": ft_sender }).to_string().as_bytes(),
                ft_storage.0,
                MIN_GAS_FOR_STORAGE_DEPOSIT,
                GasWeight(1)
            );

            // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
            // Call the function with the min GAS and then attach 1/2 of the unspent GAS to the call
            env::promise_batch_action_function_call_weight(
                batch_ft_promise_id,
                "ft_transfer",
                json!({ "receiver_id": ft_sender, "amount": ft_balance, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                1,
                MIN_GAS_FOR_FT_TRANSFER,
                GasWeight(1)
            );

            // Return the result of the batch as the return of the function
            env::promise_return(batch_ft_promise_id);
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
        // Sender of the NFT
        nft_sender: AccountId,
        // Contract where the NFT is stored
        nft_contract: AccountId,
        // Token ID for the NFT
        token_id: String, 
    ) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

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
        // Only send the NFT to the new account if the claim was successful. We return the NFT if it wasn't successful in the else case.
        if claim_succeeded {
            // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
            ext_nft_contract::ext(nft_contract.clone())
                // Call nft transfer with the min GAS and 1 yoctoNEAR. 1/2 unspent GAS will be added on top
                .with_static_gas(MIN_GAS_FOR_SIMPLE_NFT_TRANSFER)
                .with_attached_deposit(1)
                .nft_transfer(
                    account_id.clone(), 
                    token_id.clone(),
                    None,
                    Some("Linkdropped NFT".to_string()),
                )
            // We then resolve the promise and call nft_resolve_transfer on our own contract
            .then(
                // Call resolve transfer with the min GAS and no deposit. 1/2 unspent GAS will be added on top
                Self::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_RESOLVE_TRANSFER)
                    .nft_resolve_transfer(
                        token_id,
                        nft_sender,
                        nft_contract,
                    )
            );
        } else {
            // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
            ext_nft_contract::ext(nft_contract)
                // Call nft transfer with the min GAS and 1 yoctoNEAR. all unspent GAS will be added on top
                .with_static_gas(MIN_GAS_FOR_SIMPLE_NFT_TRANSFER)
                .with_attached_deposit(1)
                .nft_transfer(
                    nft_sender, 
                    token_id,
                    None,
                    Some("Linkdropped NFT".to_string()),
                );
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

        // Default amount to refund to be everything except balance (and FC deposit) and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + storage_used.0 - BURNT_GAS;
        
        env::log_str(&format!("Refund Amount: {}, Access Key Allowance: {}, Access Key Storage: {}, Storage Used: {}, Burnt GAS: {}", yocto_to_near(amount_to_refund), yocto_to_near(ACCESS_KEY_ALLOWANCE), yocto_to_near(ACCESS_KEY_STORAGE), yocto_to_near(storage_used.0), yocto_to_near(BURNT_GAS)));

        // If not successful, the balance and deposit is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccessful. Refunding linkdrop balance: {} and deposit: {}", balance.0, deposit.0));
            amount_to_refund += balance.0 + deposit.0
        }

        /* 
            If the claim is not successful, we should always refund. The only case where we don't refund is
            if the claim was successful and the user specified that the refund should go into the
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
        // Only call the function if the claim was successful.
        if claim_succeeded {
            let mut final_args = args.clone();

            // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
            if let Some(account_field) = claimed_account_field {
                final_args.insert_str(final_args.len()-1, &format!(",\"{}\":\"{}\"", account_field, account_id));
                env::log_str(&format!("Adding claimed account ID to specified field: {:?} in args: {:?}", account_field, args));
            }
        
            env::log_str(&format!("Attaching Total: {:?} Deposit: {:?} Should Refund?: {:?} Amount To Refund: {:?} With args: {:?}", yocto_to_near(deposit.0 + if add_refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}), yocto_to_near(deposit.0), add_refund_to_deposit.unwrap_or(false), yocto_to_near(amount_to_refund), final_args));

            // Call function with the min GAS and deposit. all unspent GAS will be added on top
            Promise::new(receiver).function_call_weight(
                method, 
                final_args.as_bytes().to_vec(), 
                // The claim is successful so attach the amount to refund to the deposit instead of refunding the funder.
                deposit.0 + if add_refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}, 
                MIN_GAS_FOR_CALLBACK_FUNCTION_CALL,
                GasWeight(1)
            );
        }

        claim_succeeded
    }
}