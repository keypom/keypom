use near_sdk::PromiseResult;

use crate::*;

#[near_bindgen]
impl Keypom {
    #[private]
    pub fn on_new_account_created(&mut self, token_id: TokenId, receiver_id: AccountId) -> PromiseOrValue<bool> {
        let successful_creation = was_account_created();

        // If the account was successfully created, we should claim the assets
        // Otherwise, we should loop through all the assets in the current use and refund the tokens
        if successful_creation {
            return PromiseOrValue::Promise(self.internal_claim_assets(token_id, receiver_id));
        }

        let initial_storage = env::storage_usage();
        let (drop_id, _) = parse_token_id(&token_id);
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");

        // Now that the callback is finished, we can remove the key info from the drop
        // Since no other functions need the key information
        if key_info.remaining_uses == 0 {
            drop.key_info_by_token_id.remove(&token_id).expect("Key not found");
            let mut is_drop_empty = false;

            // Loop through all the assets in the drop and ensure they're empty
            for asset in drop.asset_by_id.values() {
                if !asset.is_empty() {
                    is_drop_empty = true;
                    break;
                }
            }

            // Now that we've removed the key info, check if the drop is empty
            // Otherwise, re-insert the drop into state
            if drop.key_info_by_token_id.is_empty() && is_drop_empty {
                near_sdk::log!("Drop with ID: {} is now empty. Deleting.", drop_id);
                // Remove the drop from storage and clear the maps inside of it
                self.drop_by_id.remove(&drop_id);
                internal_clear_drop_storage(&mut drop);
            } else {
                near_sdk::log!("Drop with ID: {} is not empty. Re-inserting. Does have assets? {}", drop_id, !is_drop_empty);
                // Put the modified drop back in storage
                self.drop_by_id.insert(&drop_id, &drop);
            }
        }

        let final_storage = env::storage_usage();
        // Some storage was freed so we should refund the user's balance
        if final_storage < initial_storage {
            let storage_cost = (initial_storage - final_storage) as u128 * env::storage_byte_cost();
            self.internal_modify_user_balance(&drop.funder_id, storage_cost, false);
        }
        
        PromiseOrValue::Value(false)
    }

    #[private]
    pub fn on_assets_claimed(
        &mut self, 
        token_id: TokenId,
        token_ids_transferred: Vec<Option<TokenId>>
    ) -> PromiseOrValue<bool> {
        let num_promises = env::promise_results_count();

        let initial_storage = env::storage_usage();
        let (drop_id, _) = parse_token_id(&token_id);
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let KeyBehavior {assets_metadata, config: _} = drop.key_behavior_by_use.get(&cur_key_use).expect("Use number not found");
        
        // Iterate through all the promises and get the results
        let mut was_successful = true;
        let mut drop_assets_empty = true;
        for i in 0..num_promises {
            let promise_result = env::promise_result(i);
            let metadata = &assets_metadata[i as usize];
            let mut asset: InternalAsset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");

            match promise_result {
                PromiseResult::NotReady => return PromiseOrValue::Promise(
                    Self::ext(env::current_account_id())
                        .on_assets_claimed(
                            token_id,
                            token_ids_transferred
                        )
                ),
                PromiseResult::Successful(_) => {},
                PromiseResult::Failed => {
                    let mut tokens_per_use = metadata.tokens_per_use.map(|x| x.0.to_string());
                    
                    // If it's a NFT, we need to get the token ID
                    if let InternalAsset::nft(_) = &mut asset {
                        tokens_per_use = token_ids_transferred[i as usize].clone();
                    };

                    let amount_to_increment = asset.on_failed_claim(&tokens_per_use);
                    self.internal_modify_user_balance(&drop.funder_id, amount_to_increment, false);
                    // Re-insert into storage
                    drop.asset_by_id.insert(&metadata.asset_id, &asset);

                    was_successful = false;
                }

            }

            if !asset.is_empty() {
                drop_assets_empty = false;
            }
        }

        // Potential error -> because we only re-insert the drop at the end of the loop, what happens
        // If the promise result is not ready? Do we lose all the modifications that we made to the drop?
        self.drop_by_id.insert(&drop_id, &drop);

        // Now that the callback is finished, we can remove the key info from the drop
        // Since no other functions need the key information
        if key_info.remaining_uses == 0 {
            drop.key_info_by_token_id.remove(&token_id).expect("Key not found");

            // Now that we've removed the key info, check if the drop is empty
            // Otherwise, re-insert the drop into state
            if drop.key_info_by_token_id.is_empty() && drop_assets_empty {
                near_sdk::log!("Drop with ID: {} is now empty. Deleting.", drop_id);
                // Remove the drop from storage and clear the maps inside of it
                self.drop_by_id.remove(&drop_id);
                internal_clear_drop_storage(&mut drop);
            } else {
                near_sdk::log!("Drop with ID: {} is not empty. Re-inserting. Does have assets? {}", drop_id, !drop_assets_empty);
                // Put the modified drop back in storage
                self.drop_by_id.insert(&drop_id, &drop);
            }
        }

        let final_storage = env::storage_usage();
        // Some storage was freed so we should refund the user's balance
        if final_storage < initial_storage {
            let storage_cost = (initial_storage - final_storage) as u128 * env::storage_byte_cost();
            self.internal_modify_user_balance(&drop.funder_id, storage_cost, false);
        }

        PromiseOrValue::Value(was_successful)
    }
}