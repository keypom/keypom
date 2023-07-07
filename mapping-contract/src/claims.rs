use near_sdk::{PromiseResult, ext_contract};

use crate::*;

/// Interface for account creation
#[ext_contract(ext_account_creation)]
trait ExtAccountCreation {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}

#[near_bindgen]
impl Keypom {
    pub fn claim(&mut self, account_id: AccountId) {
        let signer_pk = env::signer_account_pk();
        
        // Remove the drop ID to prevent re-entrancy attack. Key deletion happens in another block but state
        // Is written in the same block.
        let drop_id = self.drop_id_for_pk.remove(&signer_pk).expect("Drop not found");
        
        self.internal_claim_assets(drop_id, account_id, signer_pk);
    }

    pub fn create_account_and_claim(&mut self, new_account_id: AccountId, new_public_key: PublicKey) {
        let signer_pk = env::signer_account_pk();
        
        // Remove the drop ID to prevent re-entrancy attack. Key deletion happens in another block but state
        // Is written in the same block.
        let drop_id = self.drop_id_for_pk.remove(&signer_pk).expect("Drop not found");

        // First, create the zero-balance account and then, claim the assets
        ext_account_creation::ext(self.root_account.clone())
            .with_static_gas(GAS_FOR_CREATE_ACCOUNT)
            .with_unused_gas_weight(0)
            .with_attached_deposit(10000000000000000000000) // TODO: remove (needed for sandbox testing)
            .create_account(
                new_account_id.clone(),
                new_public_key
        ).then(
            // Call resolve refund with the min GAS and no attached_deposit. 1/2 unspent GAS will be added on top
            Self::ext(env::current_account_id())
                .on_claim_account_created(
                    drop_id,
                    new_account_id,
                    signer_pk
                )
        )
        .as_return(); 
    }

    #[private]
    pub fn on_claim_account_created(&mut self, drop_id: DropId, receiver_id: AccountId, signer_pk: PublicKey) {
        let successful_creation = was_account_created();

        // If the account was successfully created, we should claim the assets
        // Otherwise, we should re-insert the public key into state
        if successful_creation {
            self.internal_claim_assets(drop_id, receiver_id, signer_pk);
        } else {
            self.drop_id_for_pk.insert(&signer_pk, &drop_id);
        }
    }

    /// Internal function that loops through all assets for the given use and claims them.
    /// Should be executed in both `claim` or `create_account_and_claim`
    pub(crate) fn internal_claim_assets(&mut self, drop_id: DropId, receiver_id: AccountId, signer_pk: PublicKey) {
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_pk.remove(&signer_pk).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let KeyBehavior {assets_metadata, config: _} = drop.key_behavior_by_use.remove(&cur_key_use).expect("Use number not found");
        
        for metadata in assets_metadata {
            let mut asset: InternalAsset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            asset.claim_asset(&drop_id, &receiver_id, &metadata.tokens_per_use.map(|x| x.into()));
            drop.asset_by_id.insert(&metadata.asset_id, &asset);
        }

        // If this is the key's last remaining use, we should schedule a promise to delete it
        // Otherwise, we should add the key back into state with the updated remaining uses
        if key_info.remaining_uses == 1 {
            Promise::new(env::current_account_id()).delete_key(signer_pk);
        } else {
            self.drop_id_for_pk.insert(&signer_pk, &drop_id);
            key_info.remaining_uses -= 1;
            drop.key_info_by_pk.insert(&signer_pk, &key_info);
        }

        // If the drop is now empty, we should delete it
        // Otherwise, re-insert the drop into state
        if drop.key_info_by_pk.is_empty() {
            // Remove the drop from storage and clear the maps inside of it
            self.drop_by_id.remove(&drop_id);
            internal_clear_drop_storage(&mut drop);
        } else {
            // Put the modified drop back in storage
            self.drop_by_id.insert(&drop_id, &drop);
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