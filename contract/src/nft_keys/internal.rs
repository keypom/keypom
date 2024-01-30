use std::convert::TryFrom;

use crate::*;

#[near_bindgen]
impl Keypom {
    // Add a token to the set of tokens an owner has
    pub(crate) fn internal_add_token_to_owner(
        &mut self,
        account_id: &AccountId,
        token_id: &TokenId,
    ) {
        //get the set of tokens for the given account
        let mut tokens_set = self.tokens_per_owner.get(account_id).unwrap_or_else(|| {
            //if the account doesn't have any tokens, we create a new unordered set
            UnorderedSet::new(StorageKeys::TokensPerOwnerInner {
                //we get a new unique prefix for the collection
                account_id_hash: hash_string(&account_id.to_string()),
            })
        });

        //we insert the token ID into the set
        tokens_set.insert(token_id);

        //we insert that set for the given account ID.
        self.tokens_per_owner.insert(account_id, &tokens_set);
    }

    //remove a token from an owner (internal method and can't be called directly via CLI).
    pub(crate) fn internal_remove_token_from_owner(
        &mut self,
        account_id: &AccountId,
        token_id: &TokenId,
    ) {
        near_sdk::log!("Attemping to Transfer using Account ID: {}", account_id);
        near_sdk::log!(
            "TOKENS_FOR_OWNER contains funder?: {:?}",
            self.tokens_per_owner
                .contains_key(&AccountId::try_from("keypom.test.near".to_string()).unwrap())
        );
        //we get the set of tokens that the owner has
        let mut tokens_set = self
            .tokens_per_owner
            .get(account_id)
            //if there is no set of tokens for the owner, we panic with the following message:
            .expect("Token should be owned by sender");

        //we remove the the token_id from the set of tokens
        tokens_set.remove(token_id);

        //if the token set is now empty, we remove the owner from the tokens_per_owner collection
        if tokens_set.is_empty() {
            self.tokens_per_owner.remove(account_id);
        } else {
            //if the token set is not empty, we simply insert it back for the account ID.
            self.tokens_per_owner.insert(account_id, &tokens_set);
        }
    }

    pub(crate) fn internal_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: Option<AccountId>,
        token_id: String,
        approval_id: Option<u64>,
        new_public_key: PublicKey,
    ) -> AccountId {
        let drop_id = parse_token_id(&token_id).unwrap().0;

        // Get drop in order to get key info (and royalties if applicable)
        let mut drop = self.drop_by_id.get(&drop_id).expect("Drop not found");

        // Get key info (will overwrite mapping to new key info after)
        let key_info = drop
            .key_info_by_token_id
            .get(&token_id)
            .expect("Key info not found");
        let cur_account = env::current_account_id();
        let old_owner = key_info.owner_id.as_ref().unwrap_or(&cur_account);
        if &sender_id != old_owner {
            //if the sender doesn't equal the owner, we check if the sender is in the approval list
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

        // Remove token from old owner if there is one
        if let Some(owner) = key_info.owner_id.as_ref() {
            self.internal_remove_token_from_owner(owner, &token_id);
        }

        // Add token to new receiver if there is one
        if let Some(new_owner) = receiver_id.as_ref() {
            self.internal_add_token_to_owner(new_owner, &token_id);
        }

        // Remove old public key from mapping
        let old_pub_key = key_info.pub_key.clone();
        self.token_id_by_pk.remove(&old_pub_key);

        // Generate new key info struct
        let new_key_info = InternalKeyInfo {
            owner_id: receiver_id.clone(),
            pub_key: new_public_key.clone(),
            approved_account_ids: Default::default(),
            remaining_uses: key_info.remaining_uses,
            next_approval_id: key_info.next_approval_id,
            metadata: key_info.metadata,
            last_claimed: key_info.last_claimed,
            pw_by_use: key_info.pw_by_use,
        };

        // Reinsert key info mapping to NFT and then add token ID mapping to public key
        drop.key_info_by_token_id.insert(&token_id, &new_key_info);
        self.drop_by_id.insert(&drop_id, &drop);

        let key_exists = self.token_id_by_pk.insert(&new_public_key, &token_id);
        assert!(key_exists.is_none(), "Key already exists");

        // Default the authorized ID to be None for the logs.
        let mut authorized_id = None;
        //if the approval ID was provided, set the authorized ID equal to the sender
        if approval_id.is_some() {
            authorized_id = Some(sender_id.to_string());
        }

        // Construct the transfer log as per the events standard.
        let event_logs = vec![
            EventLog {
                standard: NFT_STANDARD_NAME.to_string(),
                version: NFT_METADATA_SPEC.to_string(),
                event: EventLogVariant::NftTransfer(vec![NftTransferLog {
                    authorized_id: authorized_id.clone(),
                    old_owner_id: old_owner.to_string(),
                    new_owner_id: receiver_id
                        .as_ref()
                        .unwrap_or(&env::current_account_id())
                        .to_string(),
                    token_ids: vec![token_id.to_string()],
                    memo: None,
                }]),
            },
            EventLog {
                standard: KEYPOM_STANDARD_NAME.to_string(),
                version: KEYPOM_STANDARD_VERSION.to_string(),
                event: EventLogVariant::KeyTransfer(TransferKeyLog {
                    authorized_id,
                    old_owner_id: old_owner.to_string(),
                    new_owner_id: receiver_id
                        .as_ref()
                        .unwrap_or(&env::current_account_id())
                        .to_string(),
                    old_public_key: (&old_pub_key).into(),
                    new_public_key: (&new_public_key).into(),
                    drop_id,
                    token_id,
                }),
            },
        ];

        // Delete the original key
        Promise::new(env::current_account_id()).delete_key(old_pub_key);

        // Get the allowance that should be transferred to the new key
        let mut total_cost_for_key: Balance = 0;
        let mut total_allowance_for_key: Balance = drop
            .config
            .as_ref()
            .and_then(|config| config.extra_allowance_per_key)
            .unwrap_or(U128(0))
            .0;
        get_total_costs_for_key(
            &mut total_cost_for_key,
            &mut total_allowance_for_key,
            key_info.remaining_uses,
            &drop.asset_by_id,
            &drop.asset_data_for_uses,
        );

        // Add the new key with the corresponding allowance
        Promise::new(env::current_account_id()).add_access_key(
            new_public_key.clone(),
            NearToken::from_yoctonear(total_allowance_for_key),
            env::current_account_id(),
            ACCESS_KEY_METHOD_NAMES.to_string(),
        );

        // Log the transfer events
        log_events(event_logs);

        old_owner.clone()
    }
}

