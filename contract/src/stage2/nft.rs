use crate::*;

#[near_bindgen]
impl Keypom {
    pub fn nft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        msg: DropIdJson,
    ) -> PromiseOrValue<bool> {
        let contract_id = env::predecessor_account_id();

        let mut drop = self.drop_for_id.get(&msg.0).expect("No drop found for ID");
        if let DropType::nft(mut nft_data) = drop.drop_type {
            let mut token_ids = nft_data.token_ids;

            require!(
                nft_data.sender_id == sender_id && nft_data.contract_id == contract_id,
                "NFT data must match what was sent"
            );

            // Measure the storage by inserting this info and ensure the drop funder has enough balance
            let initial_storage = env::storage_usage();

            // Push the token ID to the back of the vector
            token_ids.push(&token_id);

            // Re-insert the token IDs into the NFT Data struct
            nft_data.token_ids = token_ids;

            // Increment the uses registered
            drop.registered_uses += 1;
            near_sdk::log!("drop.registered_uses {}", drop.registered_uses);

            // Add the nft data back with the updated set
            drop.drop_type = DropType::nft(nft_data);

            // Insert the drop with the updated data
            self.drop_for_id.insert(&msg.0, &drop);

            // Calculate the storage for inserting the token and subtract from funder's balance
            let final_storage = env::storage_usage();
            let total_required_storage =
                Balance::from(final_storage - initial_storage) * env::storage_byte_cost();

            let mut drop_funder_balance = self.user_balances.get(&drop.owner_id).unwrap_or(0);
            drop_funder_balance -= total_required_storage;
            near_sdk::log!(
                "Subtracting {} from funder to cover storage. New balance is {}",
                total_required_storage,
                drop_funder_balance
            );
            self.user_balances
                .insert(&drop.owner_id, &drop_funder_balance);
        } else {
            env::panic_str("drop type isn't NFT");
        }

        // Everything went well and we don't need to return the token.
        PromiseOrValue::Value(false)
    }

    #[private]
    /// self callback checks if NFT was successfully transferred to the new account. If yes, do nothing. If no, refund original sender
    pub fn nft_resolve_refund(&mut self, drop_id: U128, token_ids: Vec<String>) -> bool {
        let used_gas = env::used_gas();
        let prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "Beginning of resolve refund used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        // If not successful, the length of the token IDs needs to be added back to the drop.
        if !transfer_succeeded {
            let mut drop = self.drop_for_id.get(&drop_id.0).unwrap();
            drop.registered_uses += token_ids.len() as u64;

            if let DropType::nft(nft_data) = &mut drop.drop_type {
                // Loop through and add token IDs back into the vector
                for token in &token_ids {
                    nft_data.token_ids.push(token);
                }
            };
            self.drop_for_id.insert(&drop_id.0, &drop);

            near_sdk::log!(
                "Transfer failed. Adding {} back to drop's keys registered and pushing all token IDs back",
                token_ids.len() as u64
            );

            return false;
        }

        true
    }

    #[private]
    /// self callback checks if NFT was successfully transferred to the new account. If yes, do nothing. If no, refund original sender
    pub fn nft_resolve_transfer(
        &mut self,
        token_id: String,
        token_sender: AccountId,
        token_contract: AccountId,
    ) -> bool {
        let mut used_gas = env::used_gas();
        let mut prepaid_gas = env::prepaid_gas();

        near_sdk::log!(
            "Beginning of resolve transfer used gas: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        used_gas = env::used_gas();
        prepaid_gas = env::prepaid_gas();
        near_sdk::log!(
            "Before refunding token sender in resolve transfer: {:?} prepaid gas: {:?}",
            used_gas.0,
            prepaid_gas.0
        );

        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !transfer_succeeded {
            near_sdk::log!("Attempt to transfer the new account was unsuccessful. Sending the NFT to the original sender.");
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

    // Internal method_name for transfer NFTs. Whether the claim was successful or not is passed in
    pub(crate) fn internal_nft_transfer(
        &mut self,
        claim_succeeded: bool,
        contract_id: AccountId,
        token_id: String,
        sender_id: AccountId,
        account_id: AccountId,
    ) {
        /*
            Non Fungible Tokens
        */
        // Only send the NFT to the new account if the claim was successful. We return the NFT if it wasn't successful in the else case.
        if claim_succeeded {
            // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
            ext_nft_contract::ext(contract_id.clone())
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
                    // Call resolve transfer with the min GAS and no attached_deposit. 1/2 unspent GAS will be added on top
                    Self::ext(env::current_account_id())
                        .with_static_gas(MIN_GAS_FOR_RESOLVE_TRANSFER)
                        .nft_resolve_transfer(token_id, sender_id, contract_id),
                );
        } else {
            // CCC to the NFT contract to transfer the token to the new account. If this is unsuccessful, we transfer to the original token sender in the callback.
            ext_nft_contract::ext(contract_id)
                // Call nft transfer with the min GAS and 1 yoctoNEAR. all unspent GAS will be added on top
                .with_static_gas(MIN_GAS_FOR_SIMPLE_NFT_TRANSFER)
                .with_attached_deposit(1)
                .nft_transfer(
                    sender_id,
                    token_id,
                    None,
                    Some("Linkdropped NFT".to_string()),
                );
        }
    }
}
