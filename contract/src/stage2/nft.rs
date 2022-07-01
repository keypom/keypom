use crate::*;

/// Keep track of nft data 
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct NFTData {
    pub nft_sender: AccountId,
    pub nft_contract: AccountId,
    pub longest_token_id: String,
    pub storage_for_longest: Balance,
    pub token_ids: Option<UnorderedSet<String>>,
}

#[near_bindgen]
impl DropZone {
    pub fn nft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        msg: U128,
    ) -> PromiseOrValue<bool> {
        let contract_id = env::predecessor_account_id();

        let mut drop = self.drop_for_id.get(&msg.0).expect("No drop found for ID");
        let funder_id = drop.funder_id.clone();
        let mut nft_data = drop.nft_data.expect("No NFT data found for drop");
        let storage_per_longest = nft_data.storage_for_longest;
        let mut token_ids = nft_data.token_ids.unwrap();

        require!(nft_data.nft_sender == sender_id && nft_data.nft_contract == contract_id, "NFT data must match what was sent");
        require!(token_id.len() <= nft_data.longest_token_id.len(), "token ID must be less than largest token specified");
        require!(token_ids.insert(&token_id) == true, "token ID already registered");

        // Re-insert the token IDs into the NFT Data struct 
        nft_data.token_ids = Some(token_ids);

        // Increment the keys registered
        drop.keys_registered += 1;

        env::log_str(&format!("drop.keys_registered {}", drop.keys_registered));

        // Ensure that the number of keys registered cannot exceed the drop length
        if drop.keys_registered > drop.pks.len() {
            drop.keys_registered = drop.pks.len()
        }

        // Add the nft data back with the updated set
        drop.nft_data = Some(nft_data);

        // Measure the storage cost for inserting the token to see how it compares to the largest token ID the user paid for
        let initial_storage = env::storage_usage();
        // Insert the drop with the updated data
        self.drop_for_id.insert(&msg.0, &drop);
        
        let final_storage = env::storage_usage();
        let net_storage = Balance::from(final_storage - initial_storage);
        env::log_str(&format!("net_storage {}", yocto_to_near(net_storage)));


        // If the token ID frees up storage, refund the funder
        if storage_per_longest > net_storage {
            let refund_amount = (storage_per_longest - net_storage) * env::storage_byte_cost();
            env::log_str(&format!("Refunding {} for freeing up storage on smaller token ID: {}", funder_id, yocto_to_near(refund_amount)));
            let mut balance = self.user_balances.get(&funder_id).unwrap();
            balance += refund_amount;
            self.user_balances.insert(&funder_id, &balance);
            env::log_str(&format!("new user balance {}", yocto_to_near(balance)));
        }

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
