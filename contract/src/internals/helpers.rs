use crate::*;

// const GAS_PER_CCC: u128 = 500_000_000_000_000_000_000; // 5 TGas
// const RECEIPT_GAS_COST: u128 = 250_000_000_000_000_000_000; // 2.5 TGas

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_account_id(account_id: &String) -> CryptoHash {
    env::sha256_array(account_id.as_bytes())
}

// /// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
// pub(crate) fn calculate_allowance(attached_gas: &Balance) -> Balance {
//     require!(attached_gas % GAS_PER_CCC == 0, "attached GAS must be a multiple of the GAS per CCC");
//     let mul_constant =  1.03_f32.powf((attached_gas / GAS_PER_CCC) as f32);
//     let allowance = 
//     (attached_gas + RECEIPT_GAS_COST) 
// }

/// Calculate the allowance refund per key. 
pub(crate) fn calculate_allowance_refund_per_key(burnt_gas: u128, claims_per_key: u64) -> u128 {
    /*
        Allowance per key is equal to: 
        Total allowance for the key less all the burnt GAS divided by the number of claims per key
    */
    let total_allowance = BASE_ACCESS_KEY_ALLOWANCE + (claims_per_key - 1) as u128 * burnt_gas;
    let allowance_refund_per_key = (total_allowance - burnt_gas * claims_per_key as u128) / claims_per_key as u128;

    env::log_str(&format!(
        "Calculating allowance refund per key: {},
        Total allowance: {},
        Burnt GAS: {}
        Claims per key: {}",
        yocto_to_near(allowance_refund_per_key),
        yocto_to_near(total_allowance),
        yocto_to_near(burnt_gas), 
        claims_per_key
    ));

    allowance_refund_per_key
}

impl DropZone {
    /// Add a drop ID to the set of drops a funder has
    pub(crate) fn internal_add_drop_to_funder(
        &mut self,
        account_id: &AccountId,
        drop_id: &DropId,
    ) {
        //get the set of drops for the given account
        let mut drop_set = self.drop_ids_for_funder.get(account_id).unwrap_or_else(|| {
            //if the account doesn't have any drops, we create a new unordered set
            UnorderedSet::new(
                StorageKey::DropIdsForFunderInner {
                    //we get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&account_id.to_string()),
                }
            )
        });

        //we insert the drop ID into the set
        drop_set.insert(drop_id);

        //we insert that set for the given account ID. 
        self.drop_ids_for_funder.insert(account_id, &drop_set);
    }

    //remove a drop ID for a funder (internal method and can't be called directly via CLI).
    pub(crate) fn internal_remove_drop_for_funder(
        &mut self,
        account_id: &AccountId,
        drop_id: &DropId,
    ) {
        //we get the set of drop IDs that the funder has
        let mut drop_set = self
            .drop_ids_for_funder
            .get(account_id)
            //if there is no set of drops for the owner, we panic with the following message:
            .expect("No Drops found for the funder");

        //we remove the the drop ID from  the set of drops
        drop_set.remove(drop_id);

        //if the set is now empty, we remove the funder from the drop_ids_for_funder collection
        if drop_set.is_empty() {
            self.drop_ids_for_funder.remove(account_id);
        } else {
        //if the key set is not empty, we simply insert it back for the funder ID. 
            self.drop_ids_for_funder.insert(account_id, &drop_set);
        }
    }
}