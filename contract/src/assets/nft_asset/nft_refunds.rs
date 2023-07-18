use near_sdk::{PromiseResult, serde_json::json, GasWeight};

use crate::*;

/// Minimum Gas required to resolve the refund's batch of transfer promises
/// 5 TGas
const MIN_GAS_FOR_RESOLVE_REFUND: Gas = Gas(5_000_000_000_000);

#[near_bindgen]
impl Keypom {
    /// Allows users to withdraw their token IDs from the drop
    /// Returns a boolean indicating whether or not the transfer succeeded
    pub fn withdraw_nft_tokens(
        &mut self,
        drop_id: DropId,
        nft_contract_id: AccountId,
        token_ids: Option<Vec<TokenId>>,
        limit: Option<u8>
    ) -> Promise {
        // get the drop object
        let mut drop = self.drop_by_id.get(&drop_id).expect("No drop found");
        let funder_id = &drop.funder_id;

        require!(
            funder_id == &env::predecessor_account_id(),
            "Only drop funder can delete keys"
        );

        let mut asset: InternalAsset = drop.asset_by_id.get(&nft_contract_id.to_string()).expect("Asset not found");
        
        let initial_storage = env::storage_usage();
        let mut batch_transfer = Promise::new(env::current_account_id());
        let mut tokens_to_transfer = vec![];

        // Ensure asset is an NFT and then call the internal function
        if let InternalAsset::nft(nft_data) = &mut asset {
            // If token IDs were specified, use those. Otherwise, pop the first `limit` entries off the internal vector
            tokens_to_transfer = if let Some(token_ids) = token_ids {
                token_ids
            } else {
                let mut token_ids = vec![];

                // The number of tokens to transfer is either what was specified or the length of the vector
                let limit = limit.unwrap_or(nft_data.token_ids.len() as u8);
                let max_idx = nft_data.token_ids.len() - 1;
                for idx in 0..limit {
                    if let Some(token_id) = nft_data.token_ids.get(max_idx - idx as usize) {
                        token_ids.push(token_id.to_string());
                    } else {
                        break;
                    }
                }

                token_ids
            };

            batch_transfer = nft_data.nft_refund(
                &drop.funder_id, 
                tokens_to_transfer.clone()
            );
        };
        
        drop.asset_by_id.insert(&nft_contract_id.to_string(), &asset);
        self.drop_by_id.insert(&drop_id, &drop);

        near_sdk::log!("Tokens to transfer: {:?}", tokens_to_transfer);
        
        let net_storage_released = initial_storage - env::storage_usage();    
        // We then resolve the batch and call nft_resolve_refund on our own contract
        batch_transfer.then(
            // Call resolve refund with the min GAS and no attached_deposit. 1/2 unspent GAS will be added on top
            Keypom::ext(env::current_account_id())
                .with_static_gas(MIN_GAS_FOR_RESOLVE_REFUND)
                .resolve_nft_refund(
                    drop_id.to_string(), 
                    nft_contract_id.to_string(), 
                    drop.funder_id,
                    tokens_to_transfer.to_vec(),
                    net_storage_released.into()
                )
        )
    }

    #[private]
    pub fn resolve_nft_refund(
        &mut self, 
        drop_id: DropId, 
        asset_id: AssetId, 
        refund_to: AccountId,
        token_ids: Vec<TokenId>,
        storage_released: u128
    ) -> PromiseOrValue<bool> {
        // There will only ever be 1 promise because it was a batch
        let promise_result = env::promise_result(0);

        let initial_storage = env::storage_usage();
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let mut asset: InternalAsset = drop.asset_by_id.get(&asset_id).expect("Asset not found");

        let mut was_successful = true;
        match promise_result {
            PromiseResult::NotReady => return PromiseOrValue::Promise(
                Self::ext(env::current_account_id())
                    .resolve_nft_refund(
                        drop_id,
                        asset_id,
                        refund_to,
                        token_ids,
                        storage_released
                    )
            ),
            PromiseResult::Successful(_) => {
                near_sdk::log!("Refund successful.");
            },
            PromiseResult::Failed => {
                was_successful = false;
                // Ensure asset is an NFT and then add the token back to the internal vector 
                if let InternalAsset::nft(nft_data) = &mut asset {
                    near_sdk::log!("Refund failed. Adding all tokens back: {:?}", token_ids);
                    // Loop through each token and add them
                    for token_id in token_ids {
                        nft_data.add_to_token_ids(&token_id);
                    };
                };
            }

        }

        // If a transfer failed, we should re-insert the asset into storage
        // Since tokens have been set
        if !was_successful {
            drop.asset_by_id.insert(&asset_id, &asset);
            self.drop_by_id.insert(&drop_id, &drop);
        }

        // Measure the final storage now that tokens have been added back to the vector
        // We'll then take the storage that was released and subtract this new storage that was added
        let net_storage_added = env::storage_usage() - initial_storage;
        let total_refund = storage_released.checked_sub(net_storage_added as u128).unwrap_or(0);
        near_sdk::log!("Net storage added in refund: {} bytes. Net storage released: {} bytes.", net_storage_added, storage_released);

        // Refund the storage to the user
        self.internal_modify_user_balance(&drop.funder_id, total_refund * env::storage_byte_cost(), false);

        PromiseOrValue::Value(was_successful)
    }
}

impl InternalNFTData {
    /// Automatically refund a claim for fungible tokens
    /// This should refund the FTs & any storage deposits.
    pub fn nft_refund(
        &mut self,
        refund_to: &AccountId,
        token_ids: Vec<TokenId>
    ) -> Promise {
        // Create the batch promise on the NFT contract
        let mut batch_promise = Promise::new(self.contract_id.clone());

        // Loop through all the token IDs, and add to the batch promise while removing them from the vector
        for token_id in token_ids.clone() {
            let index = self.token_ids.iter().position(|x| *x == token_id).expect("token ID not found");
            near_sdk::log!("Refunding token ID {}. Idx: {} vec: {:?}", token_id, index, self.token_ids);
            self.token_ids.remove(index);

            batch_promise = batch_promise
            .function_call_weight(
                "nft_transfer".to_string(),
                json!({ "receiver_id": refund_to, "token_id": token_id, "memo": "Keypom Linkdrop" }).to_string().into(),
                1,
                MIN_GAS_FOR_NFT_TRANSFER,
                GasWeight(1),
            );
        }

        batch_promise                      
    }
}