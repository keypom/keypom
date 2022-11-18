use near_sdk::GasWeight;
use std::cmp;

use crate::*;

#[near_bindgen]
impl Keypom {
    /*
        User can pass in a vector of public keys or a drop ID.
        If a drop ID is passed in, it will auto delete up to limit.
        If limit is not specified, auto assume 100 keys from the drop.
        All keys must be from the same drop ID.

        All keys must be unregistered (NFTs / FTs refunded) for the drop.
    */
    pub fn delete_keys(
        &mut self,
        drop_id: DropIdJson,
        public_keys: Option<Vec<PublicKey>>,
        limit: Option<u8>,
        delete_on_empty: Option<bool>,
    ) {
        // Measure initial storage before doing any operations
        let initial_storage = env::storage_usage();

        // get the drop object
        let mut drop = self.drop_for_id.remove(&drop_id.0).expect("No drop found");
        let owner_id = drop.owner_id.clone();
        let drop_type = &drop.drop_type;
        require!(
            owner_id == env::predecessor_account_id(),
            "only drop funder can delete keys"
        );

        // Get the max uses per key. Default to 1 if not specified in the drop config.
        let uses_per_key = drop
            .config
            .clone()
            .and_then(|c| c.uses_per_key)
            .unwrap_or(1);

        // Get optional costs
        let mut ft_optional_costs_per_claim = 0;

        // ensure that there are no FTs or NFTs left to be refunded
        match drop_type {
            DropType::nft(_) => {
                require!(
                    drop.registered_uses == 0,
                    "NFTs must be refunded before keys are deleted"
                );
            }
            DropType::ft(data) => {
                require!(
                    drop.registered_uses == 0,
                    "FTs must be refunded before keys are deleted"
                );

                ft_optional_costs_per_claim = data.ft_storage.0;
            }
            _ => {}
        };

        // Keep track of the total refund amount
        let total_refund_amount;
        // Default the keys to use to be the public keys or an empty vector. We'll populate it if no PKs are passed in.
        let keys_to_delete;
        let mut total_allowance_left = 0;

        // Get the total number of uses and none FCs across all keys being deleted
        let mut total_num_uses_refunded = 0;
        let mut total_num_none_fcs = 0;
        let mut total_fc_deposits = 0;
        // If the user passed in public keys, loop through and remove them from the drop
        if let Some(keys) = public_keys {
            // Set the keys to delete equal to the keys passed in
            keys_to_delete = keys;

            let len = keys_to_delete.len() as u128;
            require!(
                len <= limit.unwrap_or(100) as u128,
                "cannot delete more than 100 keys at a time"
            );
            near_sdk::log!("Removing {} keys from the drop", len);

            // Loop through and remove keys
            for key in &keys_to_delete {
                // Unlink key to drop ID
                self.drop_id_for_pk.remove(key);
                // Attempt to remove the public key. panic if it didn't exist
                let key_info = drop.pks.remove(key).expect("public key must be in drop");
                if let Some(mut k) = key_info.pw_per_use {
                    k.clear();
                }

                total_num_uses_refunded += key_info.remaining_uses;

                // If the drop is FC, we need to loop through method_name data for the remaining number of
                // Uses and get the deposits left along with the total number of None FCs
                if let DropType::fc(data) = &drop.drop_type {
                    let num_fcs = data.methods.len() as u64;

                    // If there's one FC specified and more than 1 claim per key, that FC is to be used
                    // For all the uses. In this case, we need to tally all the deposits for each claim.
                    if uses_per_key > 1 && num_fcs == 1 {
                        let attached_deposit = data
                            .methods
                            .iter()
                            .next()
                            .unwrap()
                            .clone()
                            .expect("cannot have a single none function call")
                            // iterate through   all entries and sum the attached_deposit
                            .iter()
                            .fold(0, |acc, x| acc + x.attached_deposit.0);
                        near_sdk::log!(format!(
                            "Total attached_deposits for all method data: {}",
                            attached_deposit
                        )
                        .as_str());
                        total_fc_deposits += key_info.remaining_uses as u128 * attached_deposit;

                    // In the case where either there's 1 claim per key or the number of FCs is not 1,
                    // We can simply loop through and manually get this data
                    } else {
                        // We need to loop through the remaining methods. This means we should skip and start at the
                        // MAX - keys left
                        let starting_index = (uses_per_key - key_info.remaining_uses) as usize;
                        for method_name in data.methods.iter().skip(starting_index) {
                            total_num_none_fcs += method_name.is_none() as u64;

                            // If the method is not None, we need to get the attached_deposit by looping through the method datas
                            if let Some(method_data) = method_name {
                                let attached_deposit = method_data
                                    .iter()
                                    .fold(0, |acc, x| acc + x.attached_deposit.0);
                                total_fc_deposits += attached_deposit;
                            }
                        }
                    }
                }

                // Increment the allowance left by whatever is left on the key
                total_allowance_left += key_info.allowance;
            }

            // Keep track of the actual number of uses to refund
            let mut num_uses_to_refund = total_num_uses_refunded;

            // If the drop type is simple and has lazy registration, we need to figure out how many uses will be refunded
            if let DropType::simple(data) = &drop.drop_type {
                if data.lazy_register.unwrap_or(false) {
                    let num_registered = drop.registered_uses;

                    num_uses_to_refund = cmp::min(total_num_uses_refunded, num_registered);
                    let num_uses_not_refunded = total_num_uses_refunded - num_uses_to_refund;
                    drop.registered_uses = num_registered - num_uses_to_refund;

                    near_sdk::log!(
                        "Uses to refund {} Uses not to refund {} New registered uses: {}",
                        num_uses_to_refund, num_uses_not_refunded, num_registered - num_uses_to_refund
                    );
                }
            }

            // If the drop has no keys, remove it from the funder. Otherwise, insert it back with the updated keys.
            if drop.pks.len() == 0 && delete_on_empty.unwrap_or(true) {
                near_sdk::log!("Drop empty. Removing from funder. delete_on_empty: true");
                // If there are any excess uses, refund them since the drop is being deleted
                if let DropType::simple(data) = &drop.drop_type {
                    if data.lazy_register.unwrap_or(false) {
                        num_uses_to_refund += drop.registered_uses;
                        near_sdk::log!("Refunding {} excess uses", drop.registered_uses);
                    }
                }

                self.internal_remove_drop_for_funder(&owner_id, &drop_id.0);
            } else {
                near_sdk::log!("Drop non empty or delete on empty not set to true. Adding back. Len: {}. Delete on empty: {}", drop.pks.len(), delete_on_empty.unwrap_or(false));
                self.drop_for_id.insert(&drop_id.0, &drop);
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

            /*
                Required attached_deposit consists of:
                - TOTAL Storage freed
                - Total access key allowance for EACH key
                - Access key storage for EACH key
                - Balance for each key * (number of uses - uses with None for FC Data)

                Optional:
                - total FC deposits
                - FT storage registration cost for each key * uses (calculated in resolve storage calculation function)
            */
            let total_access_key_storage = ACCESS_KEY_STORAGE * len;
            let total_deposits =
                drop.deposit_per_use * (num_uses_to_refund - total_num_none_fcs) as u128;
            let total_ft_costs = ft_optional_costs_per_claim * total_num_uses_refunded as u128;

            total_refund_amount = total_storage_freed
                + total_allowance_left
                + total_access_key_storage
                + total_deposits
                + total_fc_deposits
                + total_ft_costs;

            near_sdk::log!(
                "Total refund: {}
                storage freed: {}
                allowance left: {}
                access key storage {}
                total deposits: {}
                num uses refunded: {}
                total fc deposits: {}
                total ft costs: {}
                total num uses left: {}
                total num none FCs {}
                len: {}",
                yocto_to_near(total_refund_amount),
                yocto_to_near(total_storage_freed),
                yocto_to_near(total_allowance_left),
                yocto_to_near(total_access_key_storage),
                yocto_to_near(total_deposits),
                num_uses_to_refund,
                yocto_to_near(total_fc_deposits),
                yocto_to_near(total_ft_costs),
                total_num_uses_refunded,
                total_num_none_fcs,
                len
            );
        } else {
            // If no PKs were passed in, attempt to remove limit or 100 keys at a time
            keys_to_delete = drop.pks.keys().take(limit.unwrap_or(100).into()).collect();

            let len = keys_to_delete.len() as u128;
            near_sdk::log!("Removing {} keys from the drop", len);

            // Loop through and remove keys
            for key in &keys_to_delete {
                // Unlink key to drop ID
                self.drop_id_for_pk.remove(key);
                // Attempt to remove the public key. panic if it didn't exist
                let key_info = drop.pks.remove(key).expect("public key must be in drop");
                if let Some(mut k) = key_info.pw_per_use {
                    k.clear();
                }
                total_num_uses_refunded += key_info.remaining_uses;

                // If the drop is FC, we need to loop through method_name data for the remaining number of
                // Uses and get the deposits left along with the total number of None FCs
                if let DropType::fc(data) = &drop.drop_type {
                    let num_fcs = data.methods.len() as u64;

                    // If there's one FC specified and more than 1 claim per key, that FC is to be used
                    // For all the uses. In this case, we need to tally all the deposits for each claim.
                    if uses_per_key > 1 && num_fcs == 1 {
                        let attached_deposit = data
                            .methods
                            .iter()
                            .next()
                            .unwrap()
                            .clone()
                            .expect("cannot have a single none function call")
                            // iterate through   all entries and sum the attached_deposit
                            .iter()
                            .fold(0, |acc, x| acc + x.attached_deposit.0);
                        near_sdk::log!(format!(
                            "Total attached_deposits for all method data: {}",
                            attached_deposit
                        )
                        .as_str());
                        total_fc_deposits += key_info.remaining_uses as u128 * attached_deposit;

                    // In the case where either there's 1 claim per key or the number of FCs is not 1,
                    // We can simply loop through and manually get this data
                    } else {
                        // We need to loop through the remaining methods. This means we should skip and start at the
                        // MAX - keys left
                        let starting_index = (uses_per_key - key_info.remaining_uses) as usize;
                        for method_name in data.methods.iter().skip(starting_index) {
                            total_num_none_fcs += method_name.is_none() as u64;
                            // If the method is not None, we need to get the attached_deposit by looping through the method datas
                            if let Some(method_data) = method_name {
                                let attached_deposit = method_data
                                    .iter()
                                    .fold(0, |acc, x| acc + x.attached_deposit.0);
                                total_fc_deposits += attached_deposit;
                            }
                        }
                    }
                }

                // Increment the allowance left by whatever is left on the key
                total_allowance_left += key_info.allowance;
            }

            // Keep track of the actual number of uses to refund
            let mut num_uses_to_refund = total_num_uses_refunded;

            // If the drop type is simple and has lazy registration, we need to figure out how many uses will be refunded
            if let DropType::simple(data) = &drop.drop_type {
                if data.lazy_register.unwrap_or(false) {
                    let num_registered = drop.registered_uses;

                    num_uses_to_refund = cmp::min(total_num_uses_refunded, num_registered);
                    let num_uses_not_refunded = total_num_uses_refunded - num_uses_to_refund;
                    drop.registered_uses = num_registered - num_uses_to_refund;

                    near_sdk::log!(
                        "Uses to refund {} Uses not to refund {} New registered uses: {}",
                        num_uses_to_refund, num_uses_not_refunded, num_registered - num_uses_to_refund
                    );
                }
            }

            // If the drop has no keys, remove it from the funder. Otherwise, insert it back with the updated keys.
            if drop.pks.len() == 0 && delete_on_empty.unwrap_or(true) {
                near_sdk::log!("Drop empty. Removing from funder. delete_on_empty: true");

                // If there are any excess uses, refund them since the drop is being deleted
                if let DropType::simple(data) = &drop.drop_type {
                    if data.lazy_register.unwrap_or(false) {
                        num_uses_to_refund += drop.registered_uses;
                        near_sdk::log!("Refunding {} excess uses", drop.registered_uses);
                    }
                }

                self.internal_remove_drop_for_funder(&owner_id, &drop_id.0);
            } else {
                near_sdk::log!("Drop non empty or delete on empty not set to true. Adding back. Len: {}. Delete on empty: {}", drop.pks.len(), delete_on_empty.unwrap_or(false));
                self.drop_for_id.insert(&drop_id.0, &drop);
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

            /*
                Required attached_deposit consists of:
                - TOTAL Storage freed
                - Total access key allowance for EACH key
                - Access key storage for EACH key
                - Balance for each key * (number of uses - uses with None for FC Data)

                Optional:
                - total FC deposits
                - FT storage registration cost for each key * uses (calculated in resolve storage calculation function)
            */
            let total_access_key_storage = ACCESS_KEY_STORAGE * len;
            let total_deposits =
                drop.deposit_per_use * (num_uses_to_refund - total_num_none_fcs) as u128;
            let total_ft_costs = ft_optional_costs_per_claim * total_num_uses_refunded as u128;

            total_refund_amount = total_storage_freed
                + total_allowance_left
                + total_access_key_storage
                + total_deposits
                + total_fc_deposits
                + total_ft_costs;

            near_sdk::log!(
                "Total refund: {}
                storage freed: {}
                allowance left: {}
                access key storage {}
                total deposits: {}
                num uses refunded: {}
                total fc deposits: {}
                total ft costs: {}
                total num uses left: {}
                total num none FCs {}
                len: {}",
                yocto_to_near(total_refund_amount),
                yocto_to_near(total_storage_freed),
                yocto_to_near(total_allowance_left),
                yocto_to_near(total_access_key_storage),
                yocto_to_near(total_deposits),
                num_uses_to_refund,
                yocto_to_near(total_fc_deposits),
                yocto_to_near(total_ft_costs),
                total_num_uses_refunded,
                total_num_none_fcs,
                len
            );
        }

        self.internal_modify_user_balance(&owner_id, total_refund_amount, false);

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
    pub fn refund_assets(&mut self, drop_id: DropIdJson, assets_to_refund: Option<u64>) {
        // get the drop object
        let mut drop = self.drop_for_id.get(&drop_id.0).expect("No drop found");
        let owner_id = drop.owner_id.clone();
        require!(
            owner_id == env::predecessor_account_id(),
            "only drop funder can delete keys"
        );

        // Get the number of uses registered for the drop.
        let uses_registered = drop.registered_uses;
        if uses_registered == 0 {
            near_sdk::log!("No uses registered. Nothing to refund");
            return;
        }

        // Get the uses to refund. If not specified, this is the number of uses currently registered.
        let num_to_refund = assets_to_refund.unwrap_or(uses_registered);
        require!(
            num_to_refund <= uses_registered,
            "can only refund less than or equal to the amount of keys registered"
        );

        // Decrement the drop's keys registered temporarily. If the transfer is unsuccessful, revert in callback.
        drop.registered_uses -= num_to_refund;
        self.drop_for_id.insert(&drop_id.0, &drop);

        match &mut drop.drop_type {
            DropType::nft(data) => {
                /*
                    NFTs need to be batched together. Loop through and transfer all NFTs.
                    Keys registered will be decremented and the token IDs will be removed
                    in the callback if everything is successful. If anything fails, the
                    keys registered will be added back in the callback for the drop.
                */
                let nft_batch_index = env::promise_batch_create(&data.contract_id);
                let mut token_ids: Vec<String> = vec![];

                // Loop through and pop / transfer all token IDs. If anything goes wrong, we send back all the token IDs, we popped and push them back in the callback.
                for _ in 0..num_to_refund {
                    let token_id = data.token_ids.pop().unwrap();
                    token_ids.push(token_id.clone());
                    // Send the NFTs back to the sender
                    // Call the function with the min GAS and then attach 1/5 of the unspent GAS to the call
                    env::promise_batch_action_function_call_weight(
                        nft_batch_index,
                        "nft_transfer",
                        json!({ "receiver_id": data.sender_id, "token_id": token_id, "memo": "Refund" }).to_string().as_bytes(),
                        1,
                        MIN_GAS_FOR_SIMPLE_NFT_TRANSFER,
                        GasWeight(1)
                    );
                }

                self.drop_for_id.insert(&drop_id.0, &drop);

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
                    json!({ "drop_id": drop_id, "token_ids": token_ids })
                        .to_string()
                        .as_bytes(),
                    NO_DEPOSIT,
                    MIN_GAS_FOR_RESOLVE_BATCH,
                    GasWeight(10),
                );
            }
            DropType::ft(data) => {
                // All FTs can be refunded at once. Funder responsible for registering themselves
                ext_ft_contract::ext(data.contract_id.clone())
                    // Call ft transfer with 1 yoctoNEAR. 1/2 unspent GAS will be added on top
                    .with_attached_deposit(1)
                    .ft_transfer(
                        data.sender_id.clone(),
                        U128(data.balance_per_use.0 * num_to_refund as u128),
                        None,
                    )
                    // We then resolve the promise and call nft_resolve_transfer on our own contract
                    .then(
                        // Call resolve refund with the min GAS and no attached_deposit. 1/2 unspent GAS will be added on top
                        Self::ext(env::current_account_id())
                            .ft_resolve_refund(drop_id.0, num_to_refund),
                    )
                    .as_return();
            }
            _ => env::panic_str("can only refund assets for FT and NFT drops"),
        };
    }
}
