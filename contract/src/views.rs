use crate::*;

#[near_bindgen]
impl Keypom {
    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let token_id = self.token_id_by_pk.get(&key).expect("no token ID found for key");
        let (drop_id, _) = parse_token_id(&token_id);
        let drop = self
            .drop_for_id
            .get(&drop_id)
            .expect("no drop found for drop ID");
        U128(drop.deposit_per_use)
    }

    /*
        CUSTOM
    */
    /// Query for the total supply of keys on the contract
    pub fn get_key_total_supply(&self) -> u64 {
        //return the length of the data_for_pk set
        self.token_id_by_pk.len()
    }

    /// Paginate through all active keys on the contract and return a vector of key info.
    pub fn get_keys(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<JsonKeyInfo> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        self.token_id_by_pk
            .keys()
            //skip to the index we specified in the start variable
            .skip(start as usize)
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize)
            //we'll map the public key which are strings into Drops
            .map(|pk| self.get_key_information(pk.clone()).unwrap())
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    /// Returns the JsonKeyInfo corresponding to a specific key
    #[handle_result]
    pub fn get_key_information(&self, key: PublicKey) -> Result<JsonKeyInfo, String> {
        // Return the optional key info if it exists

        let token_id = self.token_id_by_pk.get(&key).expect("no token ID found for key");
        let (drop_id, key_id) = parse_token_id(&token_id);
        let drop = self
            .drop_for_id
            .get(&drop_id)
            .expect("no drop found for drop ID");
        
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("no key info found for token ID");
        let cur_use = drop
            .config
            .as_ref()
            .and_then(|c| c.uses_per_key)
            .unwrap_or(1)
            - key_info.remaining_uses + 1;

        Ok(JsonKeyInfo {
            drop_id: U128(drop_id),
            pk: key.clone(),
            owner_id: key_info.owner_id,
            next_approval_id: key_info.next_approval_id,
            approved_account_ids: key_info.approved_account_ids,
            remaining_uses: key_info.remaining_uses,
            last_used: key_info.last_used,
            allowance: U128(key_info.allowance),
            key_id,
            cur_key_use: cur_use
        })
    }

    /// Returns the JsonKeyInfo corresponding to a specific key
    pub fn get_key_information_batch(&self, keys: Vec<PublicKey>) -> Vec<Option<JsonKeyInfo>> {
        // Iterate through each key in the vector and return the JsonKeyInfo for that key
        keys.iter()
            .map(|key| {
                let key_info = self.get_key_information(key.clone());
                if key_info.is_ok() {
                    Some(key_info.unwrap())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the JsonDrop corresponding to a drop ID. If a key is specified, it will return the drop info for that key.
    pub fn get_drop_information(
        &self,
        drop_id: Option<DropIdJson>,
        key: Option<PublicKey>,
    ) -> JsonDrop {
        // If the user doesn't specify a drop ID or a key, panic.
        if drop_id.is_none() && key.is_none() {
            env::panic_str("must specify either a drop ID or a public key");
        }

        // Set the drop ID to be what was passed in. If they didn't pass in a drop ID, get it
        let mut drop_id: u128 = drop_id.unwrap_or(DEFAULT_DROP_ID_JSON).0;

        // If the user specifies a key, use that to get the drop ID.
        if let Some(key) = key {
            let token_id = self.token_id_by_pk.get(&key).expect("no token ID found for key");
            drop_id = parse_token_id(&token_id).0;
        }

        let drop = self
            .drop_for_id
            .get(&drop_id)
            .expect("no drop found for drop ID");

        let mut json_drop = JsonDrop {
            drop_id: U128(drop_id),
            owner_id: drop.owner_id,
            deposit_per_use: U128(drop.deposit_per_use),
            simple: None,
            ft: None,
            fc: None,
            nft: None,
            config: None,
            registered_uses: drop.registered_uses,
            required_gas: drop.required_gas,
            metadata: drop.metadata.get(),
            next_key_id: drop.next_key_id,
        };

        if let Some(config) = drop.config {
            let json_config = JsonDropConfig {
                uses_per_key: config.uses_per_key,
                nft_key_behaviour: config.nft_key_behaviour,
                time: config.time,
                usage: config.usage,
                sale: config.sale.map(|sale| JsonPublicSaleConfig { 
                    max_num_keys: sale.max_num_keys, 
                    price_per_key: sale.price_per_key.map(|p| U128(p)), 
                    allowlist: None,
                    blocklist: None,
                    auto_withdraw_funds: sale.auto_withdraw_funds,
                    start: sale.start,
                    end: sale.end,
                }),
                root_account_id: config.root_account_id
            };

            json_drop.config = Some(json_config);
        }

        match drop.drop_type {
            DropType::fc(data) => {
                json_drop.fc = Some(data)
            }
            DropType::nft(data) => {
                json_drop.nft = Some(JsonNFTData {
                    contract_id: data.contract_id,
                    sender_id: data.sender_id,
                })
            }
            DropType::ft(data) => {
                json_drop.ft = Some(data)
            }
            DropType::simple(data) => {
                json_drop.simple = Some(data)
            }
        };

        json_drop
    }

    /// Returns the total supply of active keys for a given drop
    pub fn get_key_supply_for_drop(&self, drop_id: DropIdJson) -> u64 {
        // Get the drop object and return the length
        self.drop_for_id
            .get(&drop_id.0)
            .expect("no drop found")
            .key_info_by_token_id
            .len()
    }

    /// Paginate through keys in a specific drop
    pub fn get_keys_for_drop(
        &self,
        drop_id: DropIdJson,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<JsonKeyInfo> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        let drop = self.drop_for_id
            .get(&drop_id.0)
            .expect("No drop for given ID");
        
        return drop
            .key_info_by_token_id
            .iter()
            //skip to the index we specified in the start variable
            .skip(start as usize)
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize)
            //we'll map the public key which are strings into Drops
            .map(|(token_id, key_info)| {
                let key_id = parse_token_id(&token_id).1;
                let cur_use = drop
                    .config
                    .as_ref()
                    .and_then(|c| c.uses_per_key)
                    .unwrap_or(1)
                    - key_info.remaining_uses + 1;

                JsonKeyInfo {
                    drop_id,
                    pk: key_info.pub_key,
                    owner_id: key_info.owner_id,
                    next_approval_id: key_info.next_approval_id,
                    approved_account_ids: key_info.approved_account_ids,
                    remaining_uses: key_info.remaining_uses,
                    last_used: key_info.last_used,
                    allowance: U128(key_info.allowance),
                    key_id,
                    cur_key_use: cur_use
                }
            })
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    /// Returns the total supply of active drops for a given owner
    pub fn get_drop_supply_for_funder(&self, account_id: AccountId) -> u64 {
        //get the set of drops for the passed in owner
        let drops_for_owner = self.drop_ids_for_funder.get(&account_id);

        //if there is some set of drops, we'll return the length
        if let Some(drops_for_owner) = drops_for_owner {
            drops_for_owner.len()
        } else {
            //if there isn't a set of keys for the passed in account ID, we'll return 0
            0
        }
    }

    /// Return a vector of drop information for a owner
    pub fn get_drops_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<JsonDrop> {
        // Iterate through each drop ID and push JsonDrop to a vector
        let drop_ids = self.drop_ids_for_funder.get(&account_id);

        // If there are IDs, iterate and create the vector of JsonDrops otherwise return empty array.s
        if let Some(ids) = drop_ids {
            // Where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
            let start = u128::from(from_index.unwrap_or(U128(0)));

            ids.iter()
                // Skip to the index we specified in the start variable
                .skip(start as usize)
                // Take the first "limit" elements in the vector. If we didn't specify a limit, use 50
                .take(limit.unwrap_or(50) as usize)
                // Convert each ID into a JsonDrop
                .map(|id| self.get_drop_information(Some(U128(id)), None))
                // Collect all JsonDrops into a vector and return it
                .collect()
        } else {
            return vec![];
        }
    }

    /// Return the total supply of token IDs for a given drop
    pub fn get_nft_supply_for_drop(&self, drop_id: DropIdJson) -> u64 {
        let drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        if let DropType::nft(nft_data) = drop.drop_type {
            return nft_data.token_ids.len();
        } else {
            return 0;
        }
    }

    /// Paginate through token IDs in a drop
    pub fn get_nft_token_ids_for_drop(
        &self,
        drop_id: DropIdJson,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<String> {
        let drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        if let DropType::nft(nft_data) = drop.drop_type {
            let token_ids = nft_data.token_ids;

            // Where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
            let start = u128::from(from_index.unwrap_or(U128(0)));

            // Iterate through each token ID using an iterator
            token_ids
                .iter()
                //skip to the index we specified in the start variable
                .skip(start as usize)
                //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
                .take(limit.unwrap_or(50) as usize)
                //since we turned the keys into an iterator, we need to turn it back into a vector to return
                .collect()
        } else {
            return vec![];
        }
    }

    /// Check whether a user can add keys to a public sale or not
    pub fn can_user_add_keys(
        &self,
        drop_id: DropIdJson,
        account_id: AccountId
    ) -> bool {
        let drop = self.drop_for_id.get(&drop_id.0).expect("no drop found");
        
        if let Some(sale) = drop.config.and_then(|c| c.sale) {
            // Assert that the current time is between the start and end time
            let cur_time = env::block_timestamp();
            let desired_start = sale.start.unwrap_or(0);
            let desired_end = sale.end.unwrap_or(u64::MAX);
            if cur_time < desired_start || cur_time > desired_end {
                near_sdk::log!("Current time is not between start and end time");
                return false;
            }

            // Assert that the current account is in the allow list
            if let Some(list) = &sale.allowlist {
                if !list.contains(&account_id) {
                    near_sdk::log!("Account is not in the allow list");
                    return false;
                }
            }

            // Assert that the current account is not in the block list
            if let Some(list) = &sale.blocklist {
                if list.contains(&account_id) {
                    near_sdk::log!("Account is in the block list");
                    return false;
                }
            }   

            return true;
        } else {
            return account_id == drop.owner_id;
        }
    }

    /// Returns the current nonce on the contract
    pub fn get_next_drop_id(&self) -> u64 {
        self.next_drop_id as u64
    }

    /// Returns how many fees the contract has collected
    pub fn get_fees_collected(&self) -> U128 {
        U128(self.fees_collected)
    }

    /// Returns the current GAS price stored on the contract
    pub fn get_gas_price(&self) -> u64 {
        self.yocto_per_gas as u64
    }

    /// Returns the current linkdrop contract
    pub fn get_root_account(&self) -> String {
        self.root_account.to_string()
    }

    /// Returns the current fees associated with an account
    pub fn get_fees_per_user(&self, account_id: AccountId) -> Option<(U128, U128)> {
        // return fees per user as a U128 tuple
        self.fees_per_user
            .get(&account_id)
            .map(|fees| (U128(fees.0), U128(fees.1)))
    }

    /// Returns the current contract source metadata
    pub fn contract_source_metadata(&self) -> ContractSourceMetadata {
        self.contract_metadata.get().unwrap()
    }
}
