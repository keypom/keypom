use std::convert::TryInto;

use near_sdk::GasWeight;

use crate::*;

#[near_bindgen]
impl DropZone {
    /*
        User can pass in a vector of public keys or a drop ID.
        If a drop ID is passed in, it will auto delete up to 100
        keys from the drop. All keys must be from the same drop ID.

        All keys must be unregistered (NFTs / FTs refunded) for the drop.
    */
    pub fn delete_keys(&mut self, public_keys: Option<Vec<PublicKey>>, drop_id: DropId) {
        // Measure initial storage before doing any operations
        let initial_storage = env::storage_usage();

        // get the drop object
        let mut drop = self.drop_for_id.remove(&drop_id).expect("No drop found");
        let funder_id = drop.funder_id.clone();
        let drop_type = &drop.drop_type;
        require!(
            funder_id == env::predecessor_account_id(),
            "only drop funder can delete keys"
        );

        // ensure that there are no FTs or NFTs left to be refunded
        match drop_type {
            DropType::NFT(_) => {
                require!(
                    drop.num_claims_registered == 0,
                    "NFTs must be refunded before keys are deleted"
                );
            }
            DropType::FT(_) => {
                require!(
                    drop.num_claims_registered == 0,
                    "FTs must be refunded before keys are deleted"
                );
            }
            _ => {}
        };

        // Keep track of the total refund amount
        let total_refund_amount;
        // Default the keys to use to be the public keys or an empty vector. We'll populate it if no PKs are passed in.
        let keys_to_delete;
        let mut total_allowance_left = 0;

        // If the user passed in public keys, loop through and remove them from the drop
        if let Some(keys) = public_keys {
            // Set the keys to delete equal to the keys passed in
            keys_to_delete = keys;

            let len = keys_to_delete.len() as u128;
            require!(len <= 100, "cannot delete more than 100 keys at a time");
            near_sdk::log!("Removing {} keys from the drop", len);

            // Loop through and remove keys
            for key in &keys_to_delete {
                // Unlink key to drop ID
                self.drop_id_for_pk.remove(key);
                // Attempt to remove the public key. panic if it didn't exist
                let key_usage = drop.pks.remove(key).expect("public key must be in drop");
                // Increment the allowance left by whatever is left on the key
                total_allowance_left += key_usage.allowance;
            }

            /*
                Refund amount consists of:
                - Storage freed
                - Total access key allowance left across all keys deleted
                - Access key storage for each key * claims / key
                - Balance for linkdrop for each key * claims / key

                Optional:
                - FC deposit for each key * claims / key
                - storage for longest token ID for each key * claims / key
                - FT storage registration cost for each key * claims / key
            */
            // Get optional costs for the drop
            let optional_refund = match drop_type {
                DropType::FC(data) => data.deposit.0,
                DropType::NFT(data) => data.storage_for_longest * env::storage_byte_cost(),
                DropType::FT(data) => data.ft_storage.0,
                _ => 0,
            };

            // If the drop has no keys, remove it from the funder. Otherwise, insert it back with the updated keys.
            if drop.pks.len() == 0 {
                near_sdk::log!("Drop empty. Removing from funder");
                self.internal_remove_drop_for_funder(&funder_id, &drop_id);
            } else {
                near_sdk::log!("Drop non empty. Adding back. Len: {}", drop.pks.len());
                self.drop_for_id.insert(&drop_id, &drop);
            }

            // Calculate the storage being freed. initial - final should be >= 0 since final should be smaller than initial.
            let final_storage = env::storage_usage();
            let total_storage_freed =
                Balance::from(initial_storage - final_storage) * env::storage_byte_cost();

            total_refund_amount = total_storage_freed
                + total_allowance_left
                + (ACCESS_KEY_STORAGE + drop.balance.0 + optional_refund)
                    * drop.num_claims_registered as u128
                    * len;
        } else {
            // If no PKs were passed in, attempt to remove 100 keys at a time
            keys_to_delete = drop.pks.keys().take(100).collect();

            let len = keys_to_delete.len() as u128;
            near_sdk::log!("Removing {} keys from the drop", len);

            // Loop through and remove keys
            for key in &keys_to_delete {
                // Unlink key to drop ID
                self.drop_id_for_pk.remove(key);
                // Attempt to remove the public key. panic if it didn't exist
                let key_usage = drop.pks.remove(key).expect("public key must be in drop");
                // Increment the allowance left by whatever is left on the key
                total_allowance_left += key_usage.allowance;
            }

            /*
                Refund amount consists of:
                - Storage freed
                - Total access key allowance left across all keys deleted
                - Access key storage for each key * claims / key
                - Balance for linkdrop for each key * claims / key

                Optional:
                - FC deposit for each key * claims / key
                - storage for longest token ID for each key * claims / key
                - FT storage registration cost for each key * claims / key
            */
            // Get optional costs for the drop
            let optional_refund = match drop_type {
                DropType::FC(data) => data.deposit.0,
                DropType::NFT(data) => data.storage_for_longest * env::storage_byte_cost(),
                DropType::FT(data) => data.ft_storage.0,
                _ => 0,
            };

            // If the drop has no keys, remove it from the funder. Otherwise, insert it back with the updated keys.
            if drop.pks.len() == 0 {
                near_sdk::log!("Drop empty. Removing from funder");
                self.internal_remove_drop_for_funder(&funder_id, &drop_id);
            } else {
                near_sdk::log!("Drop non empty. Adding back. Len: {}", drop.pks.len());
                self.drop_for_id.insert(&drop_id, &drop);
            }

            // Calculate the storage being freed. initial - final should be >= 0 since final should be smaller than initial.
            let final_storage = env::storage_usage();
            let total_storage_freed =
                Balance::from(initial_storage - final_storage) * env::storage_byte_cost();
            near_sdk::log!(
                "Storage freed: {} bytes: {}",
                yocto_to_near(total_storage_freed),
                total_storage_freed
            );

            total_refund_amount = total_storage_freed
                + total_allowance_left
                + (ACCESS_KEY_STORAGE + drop.balance.0 + optional_refund)
                    * drop.num_claims_registered as u128
                    * len;
        }

        // Refund the user
        let mut cur_balance = self.user_balances.get(&funder_id).unwrap_or(0);
        near_sdk::log!(
            "Refunding user {} old balance: {}. Total allowance left: {}",
            yocto_to_near(total_refund_amount),
            yocto_to_near(cur_balance),
            yocto_to_near(total_allowance_left)
        );
        cur_balance += total_refund_amount;
        self.user_balances.insert(&funder_id, &cur_balance);

        // Loop through and delete keys
        for key in &keys_to_delete {
            // Create the batch promise
            let promise = env::promise_batch_create(&env::current_account_id());

            env::promise_batch_action_delete_key(promise, &key);

            env::promise_return(promise);
        }
    }

    /*
        Refund NFTs or FTs for a drop. User can optionally pass in a number of assets to
        refund. If not, it will try to refund all assets.
    */
    pub fn refund_assets(&mut self, drop_id: DropId, assets_to_refund: Option<u64>) {
        // get the drop object
        let mut drop = self.drop_for_id.get(&drop_id).expect("No drop found");
        let funder_id = drop.funder_id.clone();
        require!(
            funder_id == env::predecessor_account_id(),
            "only drop funder can delete keys"
        );

        // Get the number of claims registered for the drop.
        let claims_registered = drop.num_claims_registered;
        require!(claims_registered > 0, "no claims left to unregister");

        // Get the claims to refund. If not specified, this is the number of claims currently registered.
        let num_to_refund = assets_to_refund.unwrap_or(claims_registered);
        require!(
            num_to_refund <= claims_registered,
            "can only refund less than or equal to the amount of keys registered"
        );

        // Decrement the drop's keys registered temporarily. If the transfer is unsuccessful, revert in callback.
        drop.num_claims_registered -= num_to_refund;
        self.drop_for_id.insert(&drop_id, &drop);

        match drop.drop_type {
            DropType::NFT(data) => {
                /*
                    NFTs need to be batched together. Loop through and transfer all NFTs.
                    Keys registered will be decremented and the token IDs will be removed
                    in the callback if everything is successful. If anything fails, the
                    keys registered will be added back in the callback for the drop.
                */
                let nft_batch_index = env::promise_batch_create(&data.nft_contract);
                let token_ids: Vec<String> = data
                    .token_ids
                    .iter()
                    .take(num_to_refund.try_into().unwrap())
                    .collect();
                require!(
                    token_ids.len() as u64 == num_to_refund,
                    "not enough token IDs"
                );

                // TODO: delete token IDs from unordered set as mentioned in this discussion: https://github.com/mattlockyer/linkdrop/pull/6#discussion_r913345144
                // Loop through each token ID and add a transfer to the batch
                for token_id in token_ids.clone() {
                    // Send the NFTs back to the sender
                    // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
                    env::promise_batch_action_function_call_weight(
                        nft_batch_index,
                        "nft_transfer",
                        json!({ "receiver_id": data.nft_sender, "token_id": token_id, "memo": "Refund" }).to_string().as_bytes(),
                        1,
                        MIN_GAS_FOR_SIMPLE_NFT_TRANSFER,
                        GasWeight(1)
                    );
                }

                // Create the second batch promise to execute after the nft_batch_index batch is finished executing.
                // It will execute on the current account ID (this contract)
                let batch_ft_resolve_promise_id =
                    env::promise_batch_then(nft_batch_index, &env::current_account_id());

                // Execute a function call as part of the resolved promise index created in promise_batch_then
                // Callback after all NFTs were refunded
                // Call the function with the min GAS and then attach 10/(10 + num_to_refund) of the unspent GAS to the call
                env::promise_batch_action_function_call_weight(
                    batch_ft_resolve_promise_id,
                    "nft_resolve_refund",
                    json!({ "drop_id": U128(drop_id), "token_ids": token_ids })
                        .to_string()
                        .as_bytes(),
                    NO_DEPOSIT,
                    MIN_GAS_FOR_RESOLVE_BATCH,
                    GasWeight(10),
                );
            }
            DropType::FT(data) => {
                // All FTs can be refunded at once. Funder responsible for registering themselves
                ext_ft_contract::ext(data.ft_contract)
                    // Call ft transfer with 1 yoctoNEAR. 1/2 unspent GAS will be added on top
                    .with_attached_deposit(1)
                    .ft_transfer(
                        data.ft_sender,
                        U128(data.ft_balance.0 * num_to_refund as u128),
                        None,
                    )
                    // We then resolve the promise and call nft_resolve_transfer on our own contract
                    .then(
                        // Call resolve refund with the min GAS and no deposit. 1/2 unspent GAS will be added on top
                        Self::ext(env::current_account_id())
                            .ft_resolve_refund(drop_id, num_to_refund),
                    )
                    .as_return();
            }
            _ => env::panic_str("can only refund assets for FT and NFT drops"),
        };
    }
}
