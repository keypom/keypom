use crate::*;

//used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_account_id(account_id: &String) -> CryptoHash {
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

    //add a drop ID to the set of drops a funder has
    pub(crate) fn internal_add_drop_to_funder(
        &mut self,
        account_id: &AccountId,
        drop_id: &DropId,
    ) {
        //get the set of drops for the given account
        let mut drop_set = self.drops_for_funder.get(account_id).unwrap_or_else(|| {
            //if the account doesn't have any drops, we create a new unordered set
            UnorderedSet::new(
                StorageKey::DropsForFunderInner {
                    //we get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&account_id.to_string()),
                }
            )
        });

        //we insert the drop ID into the set
        drop_set.insert(drop_id);

        //we insert that set for the given account ID. 
        self.drops_for_funder.insert(account_id, &drop_set);
    }

    //remove a drop ID for a funder (internal method and can't be called directly via CLI).
    pub(crate) fn internal_remove_drop_for_funder(
        &mut self,
        account_id: &AccountId,
        drop_id: &DropId,
    ) {
        //we get the set of drop IDs that the funder has
        let mut drop_set = self
            .drops_for_funder
            .get(account_id)
            //if there is no set of drops for the owner, we panic with the following message:
            .expect("No Drops found for the funder");

        //we remove the the drop IDfrom  the set of drops
        drop_set.remove(drop_id);

        //if the set is now empty, we remove the funder from the drops_for_funder collection
        if drop_set.is_empty() {
            self.drops_for_funder.remove(account_id);
        } else {
        //if the key set is not empty, we simply insert it back for the funder ID. 
            self.drops_for_funder.insert(account_id, &drop_set);
        }
    }
}