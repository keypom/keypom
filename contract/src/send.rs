use near_sdk::Balance;

use crate::*;

#[near_bindgen]
impl LinkDropProxy {
    /*
        user has created a keypair and passes in the public key and attaches some deposit.
        this will store the account data and allow that key to call claim and create_account_and_claim
        on this contract.

        The balance is the amount of $NEAR the sender wants the linkdrop to contain.
    */
    #[payable]
    pub fn send(&mut self, 
        public_key: PublicKey,
        balance: U128,
        ft_data: Option<FTData>,
        nft_data: Option<NFTData>,
        fc_data: Option<FCData>,
    ) {
        let attached_deposit = env::attached_deposit();

        assert!(
            balance.0 >= NEW_ACCOUNT_BASE,
            "cannot have a desired account balance less than the absolute minimum for creating an account"
        );

        // Calculate the storage usage for linkdrop with the maximum U128 size
        let initial_storage = env::storage_usage();

        // Keep track of the cb info
        let mut cb_id = None;

        let pk = public_key;
        assert!(self.accounts.insert(
                &pk,
                &AccountData{
                    funder_id: env::predecessor_account_id(),
                    balance: balance,
                    storage_used: U128(340282366920938463463374607431768211455),
                    cb_id: None,
                    cb_data_sent: false
                },
            ).is_none(),
            "Account for PublicKey exists"
        );

        // Check if FT data was passed in and insert into map
        if ft_data.is_some() {
            // Ensure that if FT data is specified, NFT and FC are not
            assert!(nft_data.is_none() && fc_data.is_none(), "Cannot have multi-typed linkdrop");

            // Keep track of the callback type
            cb_id = Some(self.nonce);

            // Insert the FT data or the unique callback ID associated with the linkdrop
            self.ft.insert(
                &self.nonce, 
                &FTData { 
                    ft_contract: ft_data.clone().unwrap().ft_contract,
                    ft_sender: ft_data.clone().unwrap().ft_sender,
                    ft_balance: ft_data.clone().unwrap().ft_balance,
                    // Maximum possible storage
                    ft_storage: Some(U128(340282366920938463463374607431768211455)),
                }
            );

            // Increment nonce
            self.nonce += 1;
        }

        // Check if NFT data was passed in and insert into map
        if nft_data.is_some() {
            // Ensure that if NFT data is specified, FT and FC are not
            assert!(ft_data.is_none() && fc_data.is_none(), "Cannot have multi-typed linkdrop");
            
            // Keep track of the callback type
            cb_id = Some(self.nonce);

            // Insert passed in data into map
            self.nft.insert(
                &self.nonce,
                &nft_data.clone().unwrap()
            );

            // Increment nonce
            self.nonce += 1;
        }

        // Check if Function call data was passed in and insert into map
        if fc_data.is_some() {
            // Ensure that if FC data is specified, NFT and FT are not
            assert!(nft_data.is_none() && ft_data.is_none(), "Cannot have multi-typed linkdrop");
            // Keep track of the callback type
            cb_id = Some(self.nonce);

            // Insert passed in data into map
            self.fc.insert(
                &self.nonce,
                &fc_data.clone().unwrap()
            );
            
            // Increment nonce
            self.nonce += 1;
        }

        let final_storage = env::storage_usage();
        let required_storage = Balance::from(final_storage - initial_storage) * env::storage_byte_cost();

        /*
            Insert key back into map with proper used storage
        */
        self.accounts.insert(
            &pk,
            &AccountData{
                funder_id: env::predecessor_account_id(),
                balance: balance,
                storage_used: U128(required_storage),
                cb_id: cb_id,
                /* 
                    No need to complete step 2 (sending NFT / FTs) if the linkdrop is either regular or function call 
                    so set callback data being sent to true in that case
                */ 
                cb_data_sent: if ft_data.is_some() || nft_data.is_some() { false } else { true }
            },
        );
        
        /*
            ensure the user attached enough to cover:
            - storage on the contract
                - creating access key
                - Linkdrop data
            - access key allowance 
            - and a balance for the account (which must be greater than new account base)
            - Desired function call deposit if specified
        */
        env::log_str(&format!("Attached Deposit: {}, Access Key Storage: {}, Access Key Allowance: {}, Linkdrop Balance: {}, required storage: {}, Desired FC Attached Deposit If Applicable: {}", yocto_to_near(attached_deposit), yocto_to_near(ACCESS_KEY_STORAGE), yocto_to_near(ACCESS_KEY_ALLOWANCE), yocto_to_near(balance.0), yocto_to_near(required_storage), if fc_data.is_some() {yocto_to_near(fc_data.clone().unwrap().deposit.0)} else {0.0}));
        assert!(
            attached_deposit >= ACCESS_KEY_STORAGE + required_storage + ACCESS_KEY_ALLOWANCE + balance.0 + if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0},
            "Deposit must be large enough to cover desired balance, access key allowance, and contract storage, and function call deposit if applicable."
        );
        
        /*
            add the public key as an access key to the contract 
            which can only call claim and create_account_and_claim on this contract
        */
        Promise::new(env::current_account_id()).add_access_key(
            pk.clone(),
            ACCESS_KEY_ALLOWANCE,
            env::current_account_id(),
            ACCESS_KEY_METHOD_NAMES.to_string(),
        );

        // Check if user will attach fungible tokens
        if ft_data.is_some() {
            /*
                Get the storage required by the FT contract and ensure the user has attached enough
                deposit to cover the storage and perform refunds if they overpayed.
            */
            ext_ft_contract::storage_balance_bounds(
                ft_data.unwrap().ft_contract,
                NO_DEPOSIT,
                GAS_FOR_STORAGE_BALANCE_BOUNDS,
            ).then(ext_self::resolve_storage_check(
                vec![pk],
                env::predecessor_account_id(),
                balance,
                U128(required_storage),
                vec![cb_id.expect("callback ID expected")],
                env::current_account_id(),
                attached_deposit,
                GAS_FOR_RESOLVE_STORAGE_CHECK,
            ));
        } else if attached_deposit > balance.0 + ACCESS_KEY_ALLOWANCE + required_storage + ACCESS_KEY_STORAGE + if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0} {    
            env::log_str(&format!("Refunding User for: {}", yocto_to_near(attached_deposit - balance.0 - ACCESS_KEY_ALLOWANCE - required_storage - ACCESS_KEY_STORAGE - if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0})));
            // If the user overpaid for the desired linkdrop balance, refund them.
            Promise::new(env::predecessor_account_id()).transfer(attached_deposit - balance.0 - ACCESS_KEY_ALLOWANCE - required_storage - ACCESS_KEY_STORAGE - if fc_data.is_some() {fc_data.unwrap().deposit.0} else {0});
        }
    }

    /*
        user has created a bunch of keypairs and passed in the public keys and attached some deposit.
        this will store the account data and allow that keys to call claim and create_account_and_claim
        on this contract.

        The balance is the amount of $NEAR the sender wants each linkdrop to contain.
    */
    #[payable]
    pub fn send_multiple(
        &mut self, 
        public_keys: Vec<PublicKey>, 
        balance: U128, 
        ft_data: Option<FTData>,
        nft_data: Option<Vec<NFTData>>,
        fc_data: Option<Vec<FCData>>
    ) {
        let attached_deposit = env::attached_deposit();
        let len = public_keys.len() as u128;

        if let Some(data) = nft_data.clone() {
            assert!(
                data.len() as u128 == len,
                "Must specify NFT Data for each key"
            )
        }
        

        assert!(
            balance.0 >= NEW_ACCOUNT_BASE,
            "cannot have a desired account balance less than the absolute minimum for creating an account"
        );
        
        let current_account_id = env::current_account_id();
        let promise = env::promise_batch_create(&current_account_id);
        let mut required_storage = 0;
        let mut cb_ids = vec![];
        // Keep track of the total attached deposit across all function calls
        let mut total_attached_deposit = 0;

        let mut index = 0;
        // Loop through each public key and insert into the map and create the key
        for pk in public_keys.clone() {
            // Calculate the storage usage for linkdrop with the maximum U128 size
            let initial_storage = env::storage_usage();

            // Keep track of the cb info
            let mut cb_id = None;

            assert!(self.accounts.insert(
                &pk,
                &AccountData{
                    funder_id: env::predecessor_account_id(),
                    balance: balance,
                    storage_used: U128(340282366920938463463374607431768211455),
                    cb_id: None,
                    cb_data_sent: false
                },
            ).is_none(),
            "Account for PublicKey exists"
            );

            // Check if FT data was passed in and insert into map
            if ft_data.is_some() {
                // Ensure that if FT data is specified, NFT and FC are not
                assert!(nft_data.clone().is_none() && fc_data.clone().is_none(), "Cannot have multi-typed linkdrop");

                // Keep track of the callback type
                cb_ids.push(self.nonce);
                cb_id = Some(self.nonce);

                // Insert the FT data or the unique callback ID associated with the linkdrop
                self.ft.insert(
                    &self.nonce, 
                    &FTData { 
                        ft_contract: ft_data.clone().unwrap().ft_contract,
                        ft_sender: ft_data.clone().unwrap().ft_sender,
                        ft_balance: ft_data.clone().unwrap().ft_balance,
                        // Maximum possible storage
                        ft_storage: Some(U128(340282366920938463463374607431768211455)),
                    }
                );

                // Increment nonce
                self.nonce += 1;
            }

            // Check if NFT data was passed in and insert into map
            if nft_data.is_some() {
                // Ensure that if NFT data is specified, FT and FC are not
                assert!(ft_data.is_none() && fc_data.is_none(), "Cannot have multi-typed linkdrop");
                
                // Keep track of the callback type
                cb_ids.push(self.nonce);
                cb_id = Some(self.nonce);

                // Insert passed in data into map
                self.nft.insert(
                    &self.nonce,
                    &nft_data.clone().unwrap()[index]
                );

                // Increment nonce
                self.nonce += 1;
            }

            // Check if Function call data was passed in and insert into map
            if fc_data.is_some() {
                // Ensure that if FC data is specified, NFT and FT are not
                assert!(nft_data.is_none() && ft_data.is_none(), "Cannot have multi-typed linkdrop");
                
                // Keep track of the callback type
                cb_ids.push(self.nonce);
                cb_id = Some(self.nonce);
                total_attached_deposit = total_attached_deposit + fc_data.clone().unwrap()[index].deposit.0;

                // Insert passed in data into map
                self.fc.insert(
                    &self.nonce,
                    &fc_data.clone().unwrap()[index]
                );
                
                // Increment nonce
                self.nonce += 1;
            }
            let final_storage = env::storage_usage();
            required_storage = Balance::from(final_storage - initial_storage) * env::storage_byte_cost();
            
            /*
                Insert key back into map with proper used storage
            */
            self.accounts.insert(
                &pk,
                &AccountData{
                    funder_id: env::predecessor_account_id(),
                    balance: balance,
                    storage_used: U128(required_storage),
                    cb_id: cb_id,
                    /* 
                        No need to complete step 2 (sending NFT / FTs) if the linkdrop is either regular or function call 
                        so set callback data being sent to true in that case
                    */ 
                    cb_data_sent: if ft_data.is_some() || nft_data.is_some() { false } else { true }
                },
            );

            /*
                ensure the user attached enough to cover:
                - storage on the contract
                    - creating access key
                    - Linkdrop data
                - access key allowance 
                - and a balance for the account (which must be greater than new account base)
            */
            assert!(
                attached_deposit >= ACCESS_KEY_STORAGE + required_storage + ACCESS_KEY_ALLOWANCE + balance.0 + if fc_data.is_some() {fc_data.clone().unwrap()[index].deposit.0} else {0},
                "Deposit must be large enough to cover desired balance, access key allowance, and contract storage"
            );

            // Must assert in the loop so no access keys are made?
            env::promise_batch_action_add_key_with_function_call(
                promise, 
                &pk, 
                0, 
                ACCESS_KEY_ALLOWANCE, 
                &current_account_id, 
                ACCESS_KEY_METHOD_NAMES
            );

            index = index + 1;
        }

        /*
            ensure the user attached enough to cover:
            - storage allowance on the contract for access key and storing account data and pk
            - access key allowance 
            - and a balance for the account (which must be greater than new account base)

            this must be true for every public key passed in.
        */
        env::log_str(&format!(
            "Attached Deposit: {}, Access Key Storage: {}, Access Key Allowance: {}, Linkdrop Balance: {}, required storage: {}, total function call deposits (if applicable): {}, length: {}", 
            yocto_to_near(attached_deposit), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(ACCESS_KEY_ALLOWANCE), 
            yocto_to_near(balance.0), 
            yocto_to_near(required_storage), 
            yocto_to_near(total_attached_deposit), 
            len)
        );
        assert!(
            attached_deposit >= (ACCESS_KEY_STORAGE + required_storage + ACCESS_KEY_ALLOWANCE + balance.0 ) * len + total_attached_deposit,
            "Deposit must be large enough to cover desired balance, access key allowance, contract storage, and function call deposit (if applicable) for all keys"
        );

        env::promise_return(promise);

        // Check if user will attach fungible tokens
        if ft_data.is_some() {

            /*
                Get the storage required by the FT contract and ensure the user has attached enough
                deposit to cover the storage and perform refunds if they overpayed.
            */ 
            ext_ft_contract::storage_balance_bounds(
                ft_data.unwrap().ft_contract,
                NO_DEPOSIT,
                GAS_FOR_STORAGE_BALANCE_BOUNDS,
            ).then(ext_self::resolve_storage_check(
                public_keys,
                env::predecessor_account_id(),
                balance,
                U128(required_storage),
                cb_ids,
                env::current_account_id(),
                attached_deposit,
                GAS_FOR_RESOLVE_STORAGE_CHECK,
            ));
        } else if attached_deposit > (ACCESS_KEY_STORAGE + required_storage + ACCESS_KEY_ALLOWANCE + balance.0) * len + total_attached_deposit {
            env::log_str(&format!("Refunding User for: {}", yocto_to_near(attached_deposit - ((ACCESS_KEY_STORAGE + required_storage + ACCESS_KEY_ALLOWANCE + balance.0) * len + total_attached_deposit))));    
            // If the user overpaid for the desired linkdrop balances, refund them.
            Promise::new(env::predecessor_account_id()).transfer(attached_deposit - ((ACCESS_KEY_STORAGE + required_storage + ACCESS_KEY_ALLOWANCE + balance.0) * len + total_attached_deposit));
        }
    }
}