use near_sdk::json_types::Base64VecU8;

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
            funder_id, 
            balance, 
            storage_used, 
            cb_data_sent, 
            cb_sender, 
            cb_contract, 
            nft_id, 
            ft_balance, 
            ft_storage,
            cb_method,
            cb_args,
            cb_deposit,
            refund_to_deposit,
        ) = self.process_claim();
        
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
            /*
                EXTRA
            */
            cb_sender,
            cb_contract,
            nft_id, 
            ft_balance,
            ft_storage,
            cb_method,
            cb_args,
            cb_deposit,
            refund_to_deposit,
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

        // Ensure the user attaches enough GAS and isn't doing anything malicious / third party.
        assert!(prepaid_gas >= MIN_PREPAID_GAS_FOR_CLAIM, "Cannot attach less than the minimum amount of prepaid gas");

        env::log_str(&format!("Beginning of CAAC used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        // Delete the access key and remove / return account data, and optionally callback data.
        let (
            funder_id, 
            balance, 
            storage_used, 
            cb_data_sent, 
            cb_sender, 
            cb_contract, 
            nft_id, 
            ft_balance, 
            ft_storage,
            cb_method,
            cb_args,
            cb_deposit,
            refund_to_deposit
        ) = self.process_claim();

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
            /*
                EXTRA
            */
            cb_sender,
            cb_contract,
            nft_id, 
            ft_balance,
            ft_storage,
            cb_method,
            cb_args,
            cb_deposit,
            refund_to_deposit,
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
        /*
            EXTRA
        */
        cb_sender: Option<AccountId>,
        cb_contract: Option<AccountId>,
        nft_id: Option<String>, 
        ft_balance: Option<U128>,
        ft_storage: Option<U128>,
        cb_method: Option<String>,
        cb_args: Option<Base64VecU8>,
        cb_deposit: Option<U128>,
        refund_to_deposit: Option<bool>

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
        
        env::log_str(&format!("Refund Amount: {}, Access Key Allowance: {}, Access Key Storage: {}, Storage Used: {}, Burnt GAS: {}", yocto_to_near(amount_to_refund), yocto_to_near(ACCESS_KEY_ALLOWANCE), yocto_to_near(ACCESS_KEY_STORAGE), yocto_to_near(storage_used.0), yocto_to_near(BURNT_GAS)));

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !claim_succeeded {
            env::log_str(&format!("Claim unsuccesful. Refunding linkdrop balance as well: {}", balance.0));
            amount_to_refund += balance.0
        }

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("In on claim before refund used gas: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        /* 
            If the claim is not successful, we should always refund. The only case where we refund
            if the claim was successful is if the user specified that the refund should go into the
            deposit.

            0 0     Refund     !success  -> do refund
            0 1     Refund      success  -> do refund
            1 0     No Refund  !success  -> do refund
            1 1     No Refund   Success  -> don't do refund
        */ 
        if !claim_succeeded || (!refund_to_deposit.unwrap_or(false) && claim_succeeded) {
            // Refunding
            env::log_str(&format!("Refunding funder: {:?} For amount: {:?}", funder_id, yocto_to_near(amount_to_refund)));
            Promise::new(funder_id.clone()).transfer(amount_to_refund);
        } else {
            env::log_str(&format!("Skipping the refund to funder: {:?} claim success: {:?} refund to deposit?: {:?}", funder_id, claim_succeeded, refund_to_deposit.unwrap_or(false)));
        }

        /*
            Non Fungible Tokens
        */
        if nft_id.is_some() && cb_data_sent == true {
            let nft_contract_id = cb_contract.clone().expect("no contract ID found");
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
                    cb_sender.clone().expect("no token sender associated with NFT"),
                    nft_contract_id,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_RESOLVE_TRANSFER,
                ));
            } else {
                // CCC to the NFT contract to transfer the token to the original token sender. No callback necessary.
                ext_nft_contract::nft_transfer(
                    cb_sender.clone().expect("no token sender associated with NFT"), 
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
            let ft_contract_id = cb_contract.clone().expect("no contract ID found");
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
                    json!({ "amount": amount, "token_sender": cb_sender, "token_contract": ft_contract_id }).to_string().as_bytes(),
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
                    json!({ "account_id": cb_sender }).to_string().as_bytes(),
                    amount.0,
                    GAS_FOR_STORAGE_DEPOSIT
                );

                // Send the fungible tokens (after the storage deposit is finished since these run sequentially)
                env::promise_batch_action_function_call(
                    batch_ft_promise_id,
                    "ft_transfer",
                    json!({ "receiver_id": cb_sender, "amount": amount, "memo": "Linkdropped FT Tokens" }).to_string().as_bytes(),
                    1,
                    GAS_FOR_FT_TRANSFER
                );

                // Return the result of the batch as the return of the function
                env::promise_return(batch_ft_promise_id);
            }
            
        }

        /*
            Function Calls
        */
        if cb_method.is_some() && cb_args.is_some() && cb_deposit.is_some() && cb_data_sent == true {
            // Only call the function if the claim was successful. If not, refund the callback sender for the callback deposit. 
            let deposit = cb_deposit.unwrap();

            if claim_succeeded {
                let receiver_id = cb_contract.expect("no callback contract specified");
                let method = cb_method.unwrap();
                let args = cb_args.unwrap();

                env::log_str(&format!("Attaching Total: {:?} Deposit: {:?} Should Refund?: {:?} Amount To Refund: {:?}", yocto_to_near(deposit.0 + if refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}), yocto_to_near(deposit.0), refund_to_deposit.unwrap_or(false), yocto_to_near(amount_to_refund)));

                Promise::new(receiver_id).function_call(
                    method, 
                    args.0, 
                    // The claim is successful so attach the amount to refund to the deposit instead of refunding the funder.
                    deposit.0 + if refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}, 
                    GAS_FOR_CALLBACK_FUNCTION_CALL
                );

            } else {
                env::log_str(&format!("Claim unsuccessful. Refunding callback deposit as well: {}", yocto_to_near(deposit.0)));
                // Refunding
                Promise::new(funder_id).transfer(deposit.0);
            }
            

        }

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();

        env::log_str(&format!("End of on claim function: {:?} prepaid gas: {:?}", used_gas.0 / ONE_GIGGA_GAS, prepaid_gas.0 / ONE_GIGGA_GAS));

        claim_succeeded
    }

    /// Internal method for deleting the used key and removing / returning account data.
    fn process_claim(&mut self) -> (
        AccountId, // Funder ID
        U128, // Linkdrop Balance
        U128, // Storage used
        bool, // CB Data Sent
        /*
            EXTRA
        */
        Option<AccountId>, // Callback sender
        Option<AccountId>, // Callback contract
        Option<String>, // NFT Token ID
        Option<U128>, // FT Balance to send
        Option<U128>, // FT Storage to pay
        Option<String>, // Callback method to call
        Option<Base64VecU8>, // Callback arguments to pass
        Option<U128>, // Callback deposit to attach
        Option<bool>, // Should the refund go to the deposit of the function call
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

        let funder_id = account_data.funder_id;
        let balance = account_data.balance;
        let cb_data_sent = account_data.cb_data_sent;
        let storage_used = account_data.storage_used;

        /*
            EXTRA
        */
        let mut cb_sender = None;
        let mut cb_contract = None;
        let mut nft_id = None;
        let mut ft_balance = None;
        let mut ft_storage = None;
        let mut cb_method = None;
        let mut cb_args = None;
        let mut cb_deposit = None;
        let mut refund_to_deposit = None;

        // If the linkdrop has a callback ID, return the specific callback info. Otherwise, return only account data. 
        if let Some(nonce) = account_data.cb_id {
            let cb_type = account_data.cb_type.unwrap();

            // Check for the specific callback type and return the info.
            match cb_type {
                CBType::NFT => {
                    let nft_data = self.nft.remove(&nonce).expect("No NFT data found for the public key");
                    cb_sender = Some(nft_data.nft_sender);
                    cb_contract = Some(nft_data.nft_contract);
                    nft_id = Some(nft_data.nft_token_id);
                },
                CBType::FT => {
                    let ft_data = self.ft.remove(&nonce).expect("No FT data found for the public key");
                    cb_sender = Some(ft_data.ft_sender);
                    cb_contract = Some(ft_data.ft_contract);
                    ft_balance = Some(ft_data.ft_balance);
                    ft_storage = Some(ft_data.ft_storage.expect("FT storage missing"));
                },
                CBType::FC => {
                    let fc_data = self.fc.remove(&nonce).expect("No FC data found for the public key");
                    cb_contract = Some(fc_data.receiver);
                    cb_method = Some(fc_data.method);
                    cb_args = Some(fc_data.args);
                    cb_deposit = Some(fc_data.deposit);
                    refund_to_deposit = fc_data.refund_to_deposit;
                }
            }
        }

        // Delete the key
        Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());

        // Return account data info
        (
            funder_id, 
            balance, 
            storage_used, 
            cb_data_sent, 
            cb_sender, 
            cb_contract, 
            nft_id, 
            ft_balance, 
            ft_storage,
            cb_method,
            cb_args,
            cb_deposit,
            refund_to_deposit
        )
    }
}