use near_sdk::{PromiseResult, ext_contract};

use crate::*;

/// Interface for account creation
#[ext_contract(ext_account_creation)]
trait ExtAccountCreation {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}

#[near_bindgen]
impl Keypom {
    /// Ensure re-entry protection and decrement remaining uses on a key
    /// Returns the drop ID that the key is associated with
    pub(crate) fn before_claim_logic(&mut self) -> TokenId {
        let signer_pk = env::signer_account_pk();

        // Get the key info and decrement its remaining uses.
        // If there are zero remaining uses, break the connection between
        // The public key and token ID that way the key can't be used anymore.
        // This state will be written in the current block so prevents re-entrancy attacks.
        // We shouldn't just remove the public key mapping off the bat because there might be
        // Multi use keys that submit multiple txns in the same block. Only delete if empty.
        let token_id = self
            .token_id_by_pk
            .get(&signer_pk)
            .expect("No drop ID found for PK");
        
        let (drop_id, _) = parse_token_id(&token_id);
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        key_info.remaining_uses -= 1;
        if key_info.remaining_uses == 0 {
            // Delete everything except the token ID -> key info mapping since we need the key info in callbacks
            self.token_id_by_pk.remove(&signer_pk);
            Promise::new(env::current_account_id()).delete_key(signer_pk);
        }

        drop.key_info_by_token_id.insert(&token_id, &key_info);
        self.drop_by_id.insert(&drop_id, &drop);

        token_id
    }

    /// Internal function that loops through all assets for the given use and claims them.
    /// Should be executed in both `claim` or `create_account_and_claim`
    /// Once all assets are claimed, a cross-contract call is fired to `on_assets_claimed`
    pub(crate) fn internal_claim_assets(&mut self, token_id: TokenId, receiver_id: AccountId) -> Promise {
        let (drop_id, _) = parse_token_id(&token_id);

        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let KeyBehavior {assets_metadata, config: _} = drop.key_behavior_by_use.get(&cur_key_use).expect("Use number not found");
        
        //let promises;
        let mut promises = Vec::new();
        let mut token_ids_transferred = Vec::new();
        for metadata in assets_metadata {
            let mut asset: InternalAsset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            
            // We need to keep track of all the NFT token IDs in order to potentially perform refunds
            if let InternalAsset::nft(data) = &asset {
                token_ids_transferred.push(data.get_next_token_id());
            } else {
                token_ids_transferred.push(None);
            }

            // Some cases may result in no promise index (i.e not enough balance)
            promises.push(asset.claim_asset(&receiver_id, &metadata.tokens_per_use.map(|x| x.into())));
            
            drop.asset_by_id.insert(&metadata.asset_id, &asset);
        }

        // Put the modified drop back in storage
        self.drop_by_id.insert(&drop_id, &drop);

        let resolve = promises.into_iter().reduce(|a, b| a.and(b)).expect("empty promises");
        near_sdk::log!("Right before .then");
        resolve.then(
            Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_RESOLVE_ASSET_CLAIM)
                .on_assets_claimed(
                    token_id,
                    token_ids_transferred
                )
        )
    }

    /// Check if key is empty and perform cleanup if it is
    /// This includes removing the drop if it now has 0 keys and no refundable assets
    pub(crate) fn handle_key_cleanup(
        &mut self,
        drop: &mut InternalDrop,
        key_info: &InternalKeyInfo,
        token_id: &TokenId,
        drop_id: &DropId,
        is_drop_empty: bool,
        initial_storage: u64
    ) {
        // Now that the callback is finished, we can remove the key info from the drop
        // Since no other functions need the key information
        if key_info.remaining_uses == 0 {
            drop.key_info_by_token_id.remove(&token_id).expect("Key not found");
    
            // Now that we've removed the key info, check if the drop is empty
            // Otherwise, re-insert the drop into state
            if drop.key_info_by_token_id.is_empty() && is_drop_empty {
                near_sdk::log!("Drop with ID: {} is now empty. Deleting.", drop_id);
                // Remove the drop from storage and clear the maps inside of it
                self.drop_by_id.remove(&drop_id);
                internal_clear_drop_storage(drop);
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
    }
}

/// Returns whether or not the account was successfully created when firing the cross contract call to `create_account`
pub(crate) fn was_account_created() -> bool {
    if let PromiseResult::Successful(value) = env::promise_result(0) {
        if let Ok(account_created) = near_sdk::serde_json::from_slice::<bool>(&value) {
            //if we need don't need to return the token, we simply return true meaning everything went fine
            near_sdk::log!("create_account call returned {}", account_created);
            return account_created;
        }
    }

    false
}