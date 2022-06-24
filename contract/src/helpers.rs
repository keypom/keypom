use crate::*;

//used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_account_id(account_id: &AccountId) -> CryptoHash {
    env::sha256_array(account_id.as_bytes())
}

impl DropZone {
    /// Asserts that the cross contract call was successful. Returns the success value
    pub(crate) fn assert_success(&mut self) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
    
        assert_eq!(env::promise_results_count(), 1, "no promise result");
        matches!(env::promise_result(0), PromiseResult::Successful(_))
    }

    //add a public key to the set of keys a funder has
    pub(crate) fn internal_add_key_to_funder(
        &mut self,
        account_id: &AccountId,
        pk: &PublicKey,
    ) {
        //get the set of keys for the given account
        let mut key_set = self.keys_for_funder.get(account_id).unwrap_or_else(|| {
            //if the account doesn't have any keys, we create a new unordered set
            UnorderedSet::new(
                StorageKey::KeysPerFunderInner {
                    //we get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&account_id),
                }
            )
        });

        //we insert the public key into the set
        key_set.insert(pk);

        //we insert that set for the given account ID. 
        self.keys_for_funder.insert(account_id, &key_set);
    }

    //remove a public key for a funder (internal method and can't be called directly via CLI).
    pub(crate) fn internal_remove_key_to_funder(
        &mut self,
        account_id: &AccountId,
        pk: &PublicKey,
    ) {
        //we get the set of keys that the funder has
        let mut key_set = self
            .keys_for_funder
            .get(account_id)
            //if there is no set of keys for the owner, we panic with the following message:
            .expect("No Keys found for the funder");

        //we remove the the public key from the set of tokens
        key_set.remove(pk);

        //if the set is now empty, we remove the funder from the keys_for_funder collection
        if key_set.is_empty() {
            self.keys_for_funder.remove(account_id);
        } else {
        //if the key set is not empty, we simply insert it back for the funder ID. 
            self.keys_for_funder.insert(account_id, &key_set);
        }
    }
}