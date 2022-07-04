use crate::*;

#[near_bindgen]
impl DropZone {
    /*
        User can pass in a vector of public keys or a drop ID.
        If a drop ID is passed in, it will auto delete up to 100
        keys from the drop. All keys must be from the same drop ID
    */
    pub fn delete_keys(&mut self, 
        public_keys: Option<Vec<PublicKey>>,
        drop_id: DropId
    ) {
        // get the drop object
        let mut drop = self.drop_for_id.remove(&drop_id).expect("No drop found");
        let funder_id = drop.funder_id.clone();
        require!(funder_id == env::predecessor_account_id(), "only drop funder can delete keys");
        
        // Keep track of the total refund amount
        let total_refund_amount;
        // Default the keys to use to be the public keys or an empty vector. We'll populate it if no PKs are passed in.
        let keys_to_delete;
        
        // If the user passed in public keys, loop through and remove them from the drop
        if let Some(keys) = public_keys {
            // Set the keys to delete equal to the keys passed in
            keys_to_delete = keys;

            let len = keys_to_delete.len() as u128;
            require!(len <= 100, "cannot delete more than 100 keys at a time");
            env::log_str(&format!("Removing {} keys from the drop", len));

            // Loop through and remove keys
            for key in &keys_to_delete {
                // Unlink key to drop ID
                self.drop_id_for_pk.remove(key);
                // Attempt to remove the public key. panic if it didn't exist
                require!(drop.pks.remove(key) == true, "public key must be in drop");
            }

            /*
                Refund amount consists of:
                - Storage for each key
                - Access key allowance
                - Access key storage
                - Balance for linkdrop
                
                Optional:
                - FC deposit
                - storage for longest token ID
                - FT storage registration cost
            */ 
            let fc_cost = drop.fc_data.as_ref().map(|data| data.deposit.0).unwrap_or(0);
            let ft_cost = drop.ft_data.as_ref().map(|data| data.ft_storage.unwrap().0).unwrap_or(0);
            let nft_cost = drop.nft_data.as_ref().map(|data| data.storage_for_longest * env::storage_byte_cost()).unwrap_or(0);
            
            total_refund_amount = (drop.storage_used_per_key.0 + self.key_fee + ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + drop.balance.0 + fc_cost + ft_cost + nft_cost) * len;

            // If the drop has no keys, remove it from the funder. Otherwise, insert it back with the updated keys.
            if drop.pks.len() == 0 {
                env::log_str("Drop empty. Removing from funder");
                self.internal_remove_drop_for_funder(&funder_id, &drop_id);
            } else {
                env::log_str(&format!("Drop non empty. Adding back. Len: {}", drop.pks.len()));
                self.drop_for_id.insert(&drop_id, &drop);
            }
        } else {
            // If no PKs were passed in, attempt to remove 100 keys at a time
            keys_to_delete = drop.pks.iter().take(100).collect();
            let len = keys_to_delete.len() as u128;
            env::log_str(&format!("Removing {} keys from the drop", len));
            
            // Loop through and remove keys
            for key in &keys_to_delete {
                // Unlink key to drop ID
                self.drop_id_for_pk.remove(key);
                // Remove the PK from the unordered set
                drop.pks.remove(key);
            }

            /*
                Refund amount consists of:
                - Storage for each key
                - Access key allowance
                - Access key storage
                - Balance for linkdrop
                
                Optional:
                - FC deposit
                - storage for longest token ID
                - FT storage registration cost
            */ 
            let fc_cost = drop.fc_data.as_ref().map(|data| data.deposit.0).unwrap_or(0);
            let ft_cost = drop.ft_data.as_ref().map(|data| data.ft_storage.unwrap().0).unwrap_or(0);
            let nft_cost = drop.nft_data.as_ref().map(|data| data.storage_for_longest * env::storage_byte_cost()).unwrap_or(0);
            
            total_refund_amount = (drop.storage_used_per_key.0 + self.key_fee + ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + drop.balance.0 + fc_cost + ft_cost + nft_cost) * len;

            // If the drop has no keys, remove it from the funder. Otherwise, insert it back with the updated keys.
            if drop.pks.len() == 0 {
                env::log_str("Drop empty. Removing from funder");
                self.internal_remove_drop_for_funder(&funder_id, &drop_id);
            } else {
                env::log_str(&format!("Drop non empty. Adding back. Len: {}", drop.pks.len()));
                self.drop_for_id.insert(&drop_id, &drop);
            }
        }

        // Refund the user
        let mut cur_balance = self.user_balances.get(&funder_id).unwrap_or(0);
        env::log_str(&format!("Refunding user {} old balance: {}", yocto_to_near(total_refund_amount), yocto_to_near(cur_balance)));
        cur_balance += total_refund_amount;
        self.user_balances.insert(&funder_id, &cur_balance);

        // Loop through and delete keys
        for key in &keys_to_delete {
            // Create the batch promise
            let promise = env::promise_batch_create(&env::current_account_id());

            env::promise_batch_action_delete_key(
                promise, 
            &key, 
            );

            env::promise_return(promise);
        }
    }
}