use crate::*;

#[near_bindgen]
impl DropZone {
    
    /*
        User passes in a public key to delete from the contract. 
        All funds paid for will be refunded to the funder. Only
        the funder can be the predecessor of the call.
    */
    #[payable]
    pub fn delete(&mut self, 
        public_key: PublicKey,
    ) {
        // Get the account data for the key. If the key doesn't exist, panic.
        let account_data = self.data_for_pk
            .remove(&public_key)
            .expect("Key missing");
        
        // Ensure that the funder is the predecessor
        assert_eq!(account_data.funder_id, env::predecessor_account_id(), "only the funder can delete the key");

        // Remove the key from the set of keys mapped to the funder
        self.internal_remove_key_to_funder(&account_data.funder_id, &public_key);
        
        // If there's a Nonce, remove all occurrences of the nonce
        if let Some(nonce) = account_data.cb_id {
            self.ft.remove(&nonce);
            self.nft.remove(&nonce);
            self.fc.remove(&nonce);
        }
        
        // Delete the key
        Promise::new(env::current_account_id())
            .delete_key(public_key);

        // balance.0 + ACCESS_KEY_ALLOWANCE + required_storage + ACCESS_KEY_STORAGE + if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0}
        // ACCESS_KEY_STORAGE + required_storage.0 + ACCESS_KEY_ALLOWANCE + balance.0 + min.0
    }
}