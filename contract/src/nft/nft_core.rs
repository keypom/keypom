use std::collections::HashMap;

use crate::*;
use near_sdk::{ext_contract, Gas, PromiseResult};

const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(10_000_000_000_000);
const GAS_FOR_NFT_ON_TRANSFER: Gas = Gas(25_000_000_000_000);

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn nft_transfer(
        &mut self,
        token_id: Option<TokenId>,
        receiver_id: Option<AccountId>,
        approval_id: Option<u64>,
        memo: PublicKey,
    ) -> (AccountId, HashMap<AccountId, u32>) {
        let sender_id = env::predecessor_account_id();
        let sender_pk = env::signer_account_pk();
        let receiver_id = receiver_id.unwrap_or(env::current_account_id());

        // Token ID is either from sender PK or passed in
        let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or(token_id.expect("Token ID not provided"));
        let drop_id = parse_token_id(&token_id).0;
        
        // Get drop in order to get key info (and royalties if applicable)
        let mut drop = self.drop_for_id.get(&drop_id).expect("Drop not found");
        // Decide what methods the new access key can call
        let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
        if let Some(perms) = drop.config.as_ref().and_then(|c| c.usage.as_ref()).and_then(|u| u.permissions.as_ref()) {
            match perms {
                // If we have a config, use the config to determine what methods the access keys can call
                ClaimPermissions::claim => {
                    access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
                }
                ClaimPermissions::create_account_and_claim => {
                    access_key_method_names = ACCESS_KEY_CREATE_ACCOUNT_METHOD_NAME;
                }
            }
        }
        let nft_royalty = drop.config.and_then(|c| c.nft_key_behaviour).and_then(|b| b.nft_royalty).unwrap_or(Default::default());
        
        // Get key info (remove token ID so we can re-insert later)
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");
        
        //if the sender doesn't equal the owner, we check if the sender is in the approval list
        if sender_id != key_info.owner_id.clone() {
            //if the token's approved account IDs doesn't contain the sender, we panic
            if !key_info.approved_account_ids.contains_key(&sender_id) {
                env::panic_str("Unauthorized");
            }

            // If they included an approval_id, check if the sender's actual approval_id is the same as the one included
            if let Some(enforced_approval_id) = approval_id {
                //get the actual approval ID
                let actual_approval_id = key_info
                    .approved_account_ids
                    .get(&sender_id)
                    //if the sender isn't in the map, we panic
                    .expect("Sender is not approved account");

                //make sure that the actual approval ID is the same as the one provided
                assert_eq!(
                    actual_approval_id, &enforced_approval_id,
                    "The actual approval_id {} is different from the given approval_id {}",
                    actual_approval_id, enforced_approval_id,
                );
            }
        }

        let old_owner_id = key_info.owner_id;
        // Remove token from old owner
        self.internal_remove_token_from_owner(&old_owner_id, &token_id);
        // Add token to new receiver
        self.internal_add_token_to_owner(&receiver_id, &token_id);

        // Remove old public key from mapping
        let pub_key = key_info.pub_key.clone();
        self.token_id_by_pk.remove(&pub_key);

        let mut allowance_to_decrement = 0;
        if sender_id != env::current_account_id() {
            // Ensure the key has enough allowance
            require!(
                key_info.allowance >= env::prepaid_gas().0 as u128 * self.yocto_per_gas,
                "Not enough allowance on the key."
            );
            
            allowance_to_decrement = (env::used_gas().0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
        }

        // Generate new key info struct
        let new_key_info = KeyInfo {
            owner_id: receiver_id.clone(),
            pub_key: memo.clone(),
            approved_account_ids: Default::default(),
            remaining_uses: key_info.remaining_uses,
            last_used: key_info.last_used,
            pw_per_key: key_info.pw_per_key,
            pw_per_use: key_info.pw_per_use,
            allowance: key_info.allowance - allowance_to_decrement,
            next_approval_id: key_info.next_approval_id,
        };

        // Reinsert key info mapping to NFT and then add token ID mapping to public key
        drop.key_info_by_token_id.insert(&token_id, &new_key_info);
        self.token_id_by_pk.insert(&memo, &token_id);

        // Default the authorized ID to be None for the logs.
        let mut authorized_id = None;
        //if the approval ID was provided, set the authorized ID equal to the sender
        if approval_id.is_some() {
            authorized_id = Some(sender_id.to_string());
        }

        // Construct the transfer log as per the events standard.
        let nft_transfer_log: EventLog = EventLog {
            // Standard name ("nep171").
            standard: NFT_STANDARD_NAME.to_string(),
            // Version of the standard ("nft-1.0.0").
            version: NFT_METADATA_SPEC.to_string(),
            // The data related with the event stored in a vector.
            event: EventLogVariant::NftTransfer(vec![NftTransferLog {
                // The optional authorized account ID to transfer the token on behalf of the old owner.
                authorized_id,
                // The old owner's account ID.
                old_owner_id: old_owner_id.to_string(),
                // The account ID of the new owner of the token.
                new_owner_id: receiver_id.to_string(),
                // A vector containing the token IDs as strings.
                token_ids: vec![token_id.to_string()],
                // An optional memo to include.
                memo: None,
            }]),
        };
        // Log the serialized json.
        env::log_str(&nft_transfer_log.to_string());

        // Delete the original key
        Promise::new(env::current_account_id()).delete_key(pub_key);

        // Add the new key with the corresponding allowance
        Promise::new(env::current_account_id()).add_access_key(
            memo.clone(),
            new_key_info.allowance,
            env::current_account_id(),
            access_key_method_names.to_string(),
        );
        
        (old_owner_id, nft_royalty)
    }

    // //implementation of the transfer call method. This will transfer the NFT and call a method on the receiver_id contract
    // #[payable]
    // pub fn nft_transfer_call(
    //     &mut self,
    //     receiver_id: AccountId,
    //     token_id: TokenId,
    //     //we introduce an approval ID so that people with that approval ID can transfer the token
    //     approval_id: Option<u64>,
    //     memo: Option<String>,
    //     msg: String,
    // ) -> PromiseOrValue<bool> {
    //     //get the sender ID 
    //     let sender_id = env::predecessor_account_id();

    //     //transfer the token and get the previous token object
    //     let previous_token = self.internal_transfer(
    //         &sender_id,
    //         &receiver_id,
    //         &token_id,
    //         approval_id,
    //         memo.clone(),
    //     );

    //     //default the authorized_id to none
    //     let mut authorized_id = None; 
    //     //if the sender isn't the owner of the token, we set the authorized ID equal to the sender.
    //     if sender_id != previous_token.owner_id {
    //         authorized_id = Some(sender_id.to_string());
    //     }

    //     // Initiating receiver's call and the callback
    //     // Defaulting GAS weight to 1, no attached deposit, and static GAS equal to the GAS for nft on transfer.
    //     ext_non_fungible_token_receiver::ext(receiver_id.clone())
    //         .with_static_gas(GAS_FOR_NFT_ON_TRANSFER)
    //         .nft_on_transfer(
    //             sender_id, 
    //             previous_token.owner_id.clone(), 
    //             token_id.clone(), 
    //             msg
    //         )
    //     // We then resolve the promise and call nft_resolve_transfer on our own contract
    //     .then(
    //         // Defaulting GAS weight to 1, no attached deposit, and static GAS equal to the GAS for resolve transfer
    //         Self::ext(env::current_account_id())
    //             .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
    //             .nft_resolve_transfer(
    //                 authorized_id, // we introduce an authorized ID so that we can log the transfer
    //                 previous_token.owner_id,
    //                 receiver_id,
    //                 token_id,
    //                 previous_token.approved_account_ids,
    //                 memo, // we introduce a memo for logging in the events standard
    //             )
    //     ).into()
    // }

    //get the information for a specific token ID
    pub fn nft_token(&self, token_id: TokenId) -> Option<JsonToken> {
        let drop_id = parse_token_id(&token_id).0;
        if let Some(drop) = self.drop_for_id.get(&drop_id) {
            let config = drop.config;

            let nft_metadata = config.as_ref().and_then(|c| c.nft_key_behaviour.clone()).and_then(|b| b.nft_metadata.clone());
            let royalty = config.and_then(|c| c.nft_key_behaviour).and_then(|b| b.nft_royalty);

            if let Some(key_info) = drop.key_info_by_token_id.get(&token_id) {
                return Some(JsonToken {
                    token_id,
                    owner_id: key_info.owner_id,
                    metadata: nft_metadata.unwrap_or(TokenMetadata {
                        title: Some(String::from("Keypom Access Key")),
                        description: Some(String::from("Keypom is pretty lit")),
                        media: Some(String::from("Keypom is pretty lit")),
                        media_hash: None,
                        copies: None,
                        issued_at: None,
                        expires_at: None,
                        starts_at: None,
                        updated_at: None,
                        extra: None,
                        reference: None,
                        reference_hash: None,
                    }),
                    approved_account_ids: key_info.approved_account_ids.clone(),
                    royalty: royalty.unwrap_or_default(),
                })
            }
        }
        
        return None;
    }
}