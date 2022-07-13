use crate::*;

const GAS_PER_CCC: Gas = Gas(5_000_000_000_000); // 5 TGas
const RECEIPT_GAS_COST: Gas = Gas(2_500_000_000_000); // 2.5 TGas

/// Used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_account_id(account_id: &String) -> CryptoHash {
    env::sha256_array(account_id.as_bytes())
}

impl DropZone {
    /// Used to calculate the base allowance needed given attached GAS
    pub(crate) fn calculate_base_allowance(&self, attached_gas: Gas) -> u128 {
        // Get the number of CCCs you can make with the attached GAS
        let calls_with_gas = (attached_gas.0 / GAS_PER_CCC.0) as f32;
        // Get the constant used to pessimistically calculate the required allowance
        let pow_outcome = 1.03_f32.powf(calls_with_gas);

        // Get the required GAS based on the calculated constant
        let required_allowance = ((attached_gas.0 + RECEIPT_GAS_COST.0) as f32 * pow_outcome
            + RECEIPT_GAS_COST.0 as f32) as u128
            * self.yocto_per_gas;
        near_sdk::log!(
            "{} calls with {} attached GAS. Pow outcome: {}. Required Allowance: {}",
            calls_with_gas,
            attached_gas.0,
            pow_outcome,
            required_allowance
        );

        required_allowance
    }

    /// Add a drop ID to the set of drops a funder has
    pub(crate) fn internal_add_drop_to_funder(&mut self, account_id: &AccountId, drop_id: &DropId) {
        //get the set of drops for the given account
        let mut drop_set = self.drop_ids_for_funder.get(account_id).unwrap_or_else(|| {
            //if the account doesn't have any drops, we create a new unordered set
            UnorderedSet::new(StorageKey::DropIdsForFunderInner {
                //we get a new unique prefix for the collection
                account_id_hash: hash_account_id(&account_id.to_string()),
            })
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

    /// Internal function for executing the callback code either straight up or using `.then` for a passed in promise
    pub(crate) fn internal_execute(
        &mut self,
        drop_data: Drop,
        drop_id: DropId,
        account_id: AccountId,
        storage_freed: u128,
        token_id: Option<String>,
        storage_for_longest: Option<u128>,
        promise: Option<Promise>,
    ) {
        macro_rules! resolve_promise_or_call {
            ( $func:ident ( $($call:tt)* ) ) => {
                if let Some(promise) = promise {
                    promise.then(
                        // Call on_claim_fc with all unspent GAS + min gas for on claim. No attached deposit.
                        Self::ext(env::current_account_id())
                        .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                        .$func(
                            $($call)*
                            // Executing the function and treating it like a callback.
                            false,
                        )
                    );
                } else {
                    // We're not dealing with a promise so we simply execute the function.
                    self.$func(
                        $($call)*
                        // Executing the function and treating it NOT like a callback.
                        true,
                    );
                }
            }
        }
        // Determine what callback we should use depending on the drop type
        match drop_data.drop_type {
            DropType::FC(data) => {
                // If we're dealing with a promise, execute the callback
                resolve_promise_or_call!(on_claim_fc(
                    // Account ID that claimed the linkdrop
                    account_id,
                    // Account ID that funded the linkdrop
                    drop_data.funder_id,
                    // Balance associated with the linkdrop
                    drop_data.balance,
                    // How much storage was freed when the key was claimed
                    storage_freed,
                    // FC Data
                    data,
                    // Drop ID
                    drop_id,
                ));
            }
            DropType::NFT(data) => {
                resolve_promise_or_call!(on_claim_nft(
                    // Account ID that claimed the linkdrop
                    account_id,
                    // Account ID that funded the linkdrop
                    drop_data.funder_id,
                    // Balance associated with the linkdrop
                    drop_data.balance,
                    // How much storage was freed when the key was claimed
                    storage_freed,
                    // How much storage was prepaid to cover the longest token ID being inserted.
                    storage_for_longest.expect("no storage for longest token Id found"),
                    // Sender of the NFT
                    data.nft_sender,
                    // Contract where the NFT is stored
                    data.nft_contract,
                    // Token ID for the NFT
                    token_id.expect("no token ID found"),
                ));
            }
            DropType::FT(data) => {
                resolve_promise_or_call!(on_claim_ft(
                    // Account ID that claimed the linkdrop
                    account_id,
                    // Account ID that funded the linkdrop
                    drop_data.funder_id,
                    // Balance associated with the linkdrop
                    drop_data.balance,
                    // How much storage was freed when the key was claimed
                    storage_freed,
                    // FT Data to be used
                    data,
                ));
            }
            DropType::Simple => {
                promise.unwrap().then(
                    // Call on_claim_simple with all unspent GAS + min gas for on claim. No attached deposit.
                    Self::ext(env::current_account_id())
                        .with_static_gas(MIN_GAS_FOR_ON_CLAIM)
                        .on_claim_simple(
                            // Account ID that funded the linkdrop
                            drop_data.funder_id,
                            // Balance associated with the linkdrop
                            drop_data.balance,
                            // How much storage was freed when the key was claimed
                            storage_freed,
                        ),
                );
            }
        };
    }
}
