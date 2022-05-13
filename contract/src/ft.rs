use crate::*;

// Returned from the storage balance bounds cross contract call on the FT contract
#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalanceBounds {
    pub min: U128,
    pub max: Option<U128>,
}

#[near_bindgen]
impl LinkDropProxy {
    /// Allows users to attach fungible tokens to the Linkdrops. Must have storage recorded by this point. You can only attach one set of FTs or NFT at a time.
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: PublicKey,
    ) -> PromiseOrValue<U128> {
        let contract_id = env::predecessor_account_id();

        // No need to assert that the funder is the sender since we don't wanna enforce anything unnecessary.
        // All that matters is we've received the FTs and that they belongs to some public key.
        let AccountData {
            funder_id,
            balance,
            token_sender: _,
            token_contract,
            nft_id: _,
            ft_balance: _,
            ft_storage,
        } = self.accounts
            .get(&msg)
            .expect("Missing public key");

        // Ensure that we know the storage and it has been paid
        assert!(ft_storage.is_some());
        // Ensure that the token contract is none since we only store one set of NFT / FT data.
        assert!(token_contract.is_none(), "PK must have no external token contract.");

        // Insert the FT token amount and token contract back into the map along with the sender ID (which may differ from funder)
        self.accounts.insert(
            &msg,
            &AccountData{
                funder_id,
                balance: balance,
                token_sender: Some(sender_id),
                token_contract: Some(contract_id),
                nft_id: None,
                ft_balance: Some(amount),
                ft_storage,
            },
        );

        // Everything went well and we don't need to return any tokens
        PromiseOrValue::Value(U128(0))
    }

    /// Self callback checks if fungible tokens were successfully transferred to the new account. If yes, do nothing. If no, refund original sender
    pub fn ft_resolve_batch(
        &mut self, 
        amount: U128, 
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

        if transfer_succeeded {
            return true
        }
        // Create a new batch promise to pay storage and refund the FTs to the original sender 
        let batch_ft_promise_id = env::promise_batch_create(&token_contract);

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
            json!({ "receiver_id": token_sender, "amount": amount, "memo": "Refunding Linkdropped FT Tokens" }).to_string().as_bytes(),
            1,
            GAS_FOR_FT_TRANSFER
        );

        // Return the result of the batch as the return of the function
        env::promise_return(batch_ft_promise_id);

        false
    }

    #[payable]
    /// self callback gets the storage balance bounds and inserts that into account data for each public key passed in
    pub fn resolve_storage_check(
        &mut self, 
        public_keys: Vec<PublicKey>, 
        funder_id: AccountId,
        balance: U128 
    ) {
        let attached_deposit = env::attached_deposit();
        let len = public_keys.len() as u128;

        // Check promise result.
        let result = promise_result_as_success().unwrap_or_else(|| {
            // Refund the funder any excess $NEAR and panic which will cause generic $NEAR linkdrops to be used
            Promise::new(funder_id.clone()).transfer(attached_deposit - (balance.0 + ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE) * len);
            env::panic_str("Unsuccessful query to get storage. Refunding funder excess $NEAR and generic $NEAR linkdrop will be used.");
        });

        // Try to get the storage balance bounds from the result of the promise
		let StorageBalanceBounds{ min, max: _ } = near_sdk::serde_json::from_slice::<StorageBalanceBounds>(&result).unwrap_or_else(|_| {
            // Refund the funder any excess $NEAR and panic which will cause generic $NEAR linkdrops to be used
            Promise::new(funder_id.clone()).transfer(attached_deposit - (balance.0 + ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE) * len);
            env::panic_str("Invalid storage balance. Refunding funder excess $NEAR and generic $NEAR linkdrop will be used.");
        });

        // Ensure the user attached enough to cover the regular $NEAR linkdrops case PLUS the storage for the fungible token contract for each key
        assert!(
            attached_deposit >= attached_deposit - (balance.0 + ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE + min.0) * len,
            "Deposit must be large enough to cover desired balance, access key allowance, and contract storage"
        );

        // Loop through each public key and insert them into the map with the new FT storage
        for pk in public_keys {
            // Ensure none of the public keys exist already
            self.accounts.insert(
                    &pk,
                    &AccountData{
                        funder_id: funder_id.clone(),
                        balance,
                        token_sender: None,
                        token_contract: None,
                        nft_id: None,
                        ft_balance: None,
                        ft_storage: Some(min)
                    },
                );
        }

        if attached_deposit > (balance.0 + ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE) * len {    
            // If the user overpaid for the desired linkdrop balance, refund them.
            Promise::new(funder_id).transfer(attached_deposit - (balance.0 + ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE) * len);
        }
    }
}
