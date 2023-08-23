use near_sdk::PromiseResult;

use crate::*;

#[near_bindgen]
impl Keypom {
    #[private]
    pub fn on_new_account_created(&mut self, token_id: TokenId, receiver_id: AccountId, fc_args: UserProvidedFCArgs, new_public_key: PublicKey) -> PromiseOrValue<bool> {
        let successful_creation = was_account_created();

        // If the account was successfully created, we should claim the assets
        // Otherwise, we should loop through all the assets in the current use and refund the tokens
        if successful_creation {
            return self.internal_claim_assets(token_id, receiver_id, fc_args, Some(new_public_key));
        }

        let initial_storage = env::storage_usage();
        let (drop_id, _) = parse_token_id(&token_id).unwrap();
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        // The uses were decremented before the account creation, so we need to increment them back to get what use should be refunded
        let cur_key_use = get_key_cur_use(&drop, &key_info) - 1;
        let InternalAssetDataForUses { uses: _, config: _, assets_metadata, required_asset_gas: _ } = get_asset_data_for_specific_use(&drop.asset_data_for_uses, &cur_key_use);

        for metadata in assets_metadata {
            let amount_to_increment = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found").get_yocto_refund_amount(&metadata.tokens_per_use.map(|t| t.into()));
            self.internal_modify_user_balance(&drop.funder_id, amount_to_increment, false);
        }

        // Loop through all the assets in the drop and ensure they're empty
        let mut is_drop_empty = true;
        for asset in drop.asset_by_id.values() {
            if !asset.is_empty() {
                is_drop_empty = false;
                break;
            }
        }

        // If the key no longer has uses, it should be removed from the drop.
        // In addition, if the drop has no keys and no more refundable assets, it should be removed from storage
        self.handle_key_cleanup(
            &mut drop,
            &key_info,
            &token_id,
            &drop_id,
            is_drop_empty,
            initial_storage
        );
        
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
        let (drop_id, _) = parse_token_id(&token_id).unwrap();
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        // The uses were decremented before the claim, so we need to increment them back to get what use should be refunded
        let cur_key_use = get_key_cur_use(&drop, &key_info) - 1;
        let InternalAssetDataForUses { uses: _, config: _, assets_metadata, required_asset_gas: _ } = get_asset_data_for_specific_use(&drop.asset_data_for_uses, &cur_key_use);
        
        // Iterate through all the promises and get the results
        let mut was_successful = true;
        let mut drop_assets_empty = true;
        near_sdk::log!("ALL ASSETS METADATA: {:?}", assets_metadata.clone());
        near_sdk::log!("Number of Promise Results: {}", env::promise_results_count());
        for i in 0..env::promise_results_count() {
            match env::promise_result(i){
                PromiseResult::NotReady => return PromiseOrValue::Promise(
                    Self::ext(env::current_account_id())
                        .on_assets_claimed(
                            token_id,
                            token_ids_transferred
                        )
                ),
                PromiseResult::Successful(_) => {
                    near_sdk::log!("Asset {}: {:?}", i + 1, assets_metadata[i as usize]);
                    near_sdk::log!("Promise {} Result: {:?}", i + 1, env::promise_result(i)); 
                },
                PromiseResult::Failed => {
                    near_sdk::log!("Asset {}: {:?}", i + 1, assets_metadata[i as usize]);
                    near_sdk::log!("Promise {} Result: {:?}", i + 1, env::promise_result(i));
                    return PromiseOrValue::Promise(
                        Self::ext(env::current_account_id())
                            .on_assets_claimed(
                                token_id,
                                token_ids_transferred
                            ))    
                }
            }
        }
        for i in 0..num_promises {
            let promise_result = env::promise_result(i);
            let metadata = &assets_metadata[i as usize];
            
            match promise_result {
                PromiseResult::NotReady => return PromiseOrValue::Promise(
                    Self::ext(env::current_account_id())
                        .on_assets_claimed(
                            token_id,
                            token_ids_transferred
                        )
                ),
                PromiseResult::Successful(_) => {
                    if is_fc_asset_id(&metadata.asset_id) {
                        near_sdk::log!("FC asset claimed");
                        continue;
                    }

                    let asset: InternalAsset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");  
                    if !asset.is_empty() {
                        drop_assets_empty = false;
                    }
                },
                PromiseResult::Failed => {
                    was_successful = false;
                    near_sdk::log!("Asset claim failed on {} asset", i);
                    // If we're dealing with an FC asset, no need to perform any refunds
                    if is_fc_asset_id(&metadata.asset_id) {
                        near_sdk::log!("FC asset claimed");
                        continue;
                    }
                    
                    let mut asset: InternalAsset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");  
                    let mut tokens_per_use = metadata.tokens_per_use.map(|x| x.0.to_string());
                    
                    // If it's a NFT, we need to get the token ID
                    if let InternalAsset::nft(_) = &mut asset {
                        tokens_per_use = token_ids_transferred[i as usize].clone();
                    };

                    let amount_to_increment = asset.on_failed_claim(&tokens_per_use);
                    self.internal_modify_user_balance(&drop.funder_id, amount_to_increment, false);
                    // Re-insert into storage
                    drop.asset_by_id.insert(&metadata.asset_id, &asset); 
                    if !asset.is_empty() {
                        drop_assets_empty = false;
                    }
                }

            }
        }

        // Potential error -> because we only re-insert the drop at the end of the loop, what happens
        // If the promise result is not ready? Do we lose all the modifications that we made to the drop?
        self.drop_by_id.insert(&drop_id, &drop);

        // If the key no longer has uses, it should be removed from the drop.
        // In addition, if the drop has no keys and no more refundable assets, it should be removed from storage
        self.handle_key_cleanup(
            &mut drop,
            &key_info,
            &token_id,
            &drop_id,
            drop_assets_empty,
            initial_storage
        );

        PromiseOrValue::Value(was_successful)
    }
}