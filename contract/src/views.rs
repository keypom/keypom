use crate::*;

/// Struct to return in views to query for drop info
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonDrop {
    // Funder of this specific drop
    pub funder_id: AccountId,
    // Balance for all linkdrops of this drop
    pub balance: U128,

    // Specific data associated with this drop
    pub ft_data: Option<FTData>, 
    pub nft_data: Option<NFTData>, 
    pub fc_data: Option<FCData>,
    // How much storage was used for EACH key and not the entire drop as a whole 
    pub storage_used_per_key: U128,
    // How many keys are registered (assets such as FTs sent)
    pub keys_registered: u64,
}


/// Struct to return in views to query for specific data related to an access key.
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonKeyInfo {
    pub pk: PublicKey,
    // Funder of this specific drop
    pub funder_id: AccountId,
    // Balance for all linkdrops of this drop
    pub balance: U128,

    // Specific data associated with this drop
    pub ft_data: Option<FTData>, 
    pub nft_data: Option<NFTData>, 
    pub fc_data: Option<FCData>,
    // How much storage was used for EACH key and not the entire drop as a whole 
    pub storage_used_per_key: U128,
}

#[near_bindgen]
impl DropZone {
    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let drop_id = self.drop_id_for_pk.get(&key).expect("no drop ID found for key");
        let drop = self.drop_for_id.get(&drop_id).expect("no drop found for drop ID");
        (drop.balance.0).into()
    }

    /*
        CUSTOM
    */
    /// Query for the total supply of keys on the contract
    pub fn key_total_supply(
        &self
    ) -> U128 {
        //return the length of the data_for_pk set
        U128(self.drop_id_for_pk.len() as u128)
    }

    /// Paginate through all active keys on the contract and return a vector of key info.
    pub fn get_keys(
        &self, 
        from_index: Option<U128>, 
        limit: Option<u64>
    ) -> Vec<JsonKeyInfo> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        self.drop_id_for_pk.keys()
            //skip to the index we specified in the start variable
            .skip(start as usize) 
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize) 
            //we'll map the public key which are strings into Drops
            .map(|pk| self.get_key_information(pk.clone()))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    /// Returns the JsonKeyInfo corresponding to a specific key
    pub fn get_key_information(
        &self,
        key: PublicKey
    ) -> JsonKeyInfo {
        let drop_id = self.drop_id_for_pk.get(&key).expect("no drop ID found for key");
        let drop = self.drop_for_id.get(&drop_id).expect("no drop found for drop ID");

        JsonKeyInfo { 
            pk: key,
            funder_id: drop.funder_id,
            balance: drop.balance,
            ft_data: drop.ft_data,
            nft_data: drop.nft_data,
            fc_data: drop.fc_data,
            storage_used_per_key: drop.storage_used_per_key,
        }
    }

    /// Returns the JsonDrop corresponding to a drop ID
    pub fn get_drop_information(
        &self,
        drop_id: DropId
    ) -> JsonDrop {
        let drop = self.drop_for_id.get(&drop_id).expect("no drop found for drop ID");

        JsonDrop { 
            funder_id: drop.funder_id,
            balance: drop.balance,
            ft_data: drop.ft_data,
            nft_data: drop.nft_data,
            fc_data: drop.fc_data,
            storage_used_per_key: drop.storage_used_per_key,
            keys_registered: drop.keys_registered
        }
    }

    /// Paginate through keys in a specific drop
    pub fn get_keys_for_drop(
        &self,
        drop_id: DropId,
        from_index: Option<U128>, 
        limit: Option<u64>
    ) -> Vec<JsonKeyInfo> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        self.drop_for_id.get(&drop_id).expect("No drop for given ID").pks.iter()
            //skip to the index we specified in the start variable
            .skip(start as usize) 
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize) 
            //we'll map the public key which are strings into Drops
            .map(|pk| self.get_key_information(pk.clone()))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }
    
    /// Returns the total supply of active keys for a given funder
    pub fn key_supply_for_funder(
        &self, 
        account_id: AccountId,
    ) -> u64 {
        //get the set of drops for the passed in funder
        let drops_for_owner = self.drop_ids_for_funder.get(&account_id);
        env::log_str(&format!("Drops: {:?}", drops_for_owner));

        //if there is some set of drops, we'll iterate through and collect all the keys
        if let Some(drops_for_owner) = drops_for_owner {
            let mut supply = 0;
            for id in drops_for_owner.iter() {
                env::log_str(&format!("ID: {:?}", id));
                supply += self.drop_for_id.get(&id).unwrap().pks.len();
            }

            supply
        } else {
            //if there isn't a set of keys for the passed in account ID, we'll return 0
            0
        }
    }

    /// Returns the total supply of active drops for a given funder
    pub fn drop_supply_for_funder(
        &self, 
        account_id: AccountId,
    ) -> u64 {
        //get the set of drops for the passed in funder
        let drops_for_owner = self.drop_ids_for_funder.get(&account_id);

        //if there is some set of drops, we'll return the length
        if let Some(drops_for_owner) = drops_for_owner {
            drops_for_owner.len()
        } else {
            //if there isn't a set of keys for the passed in account ID, we'll return 0
            0
        }
    }

    /// Return a vector of drop information for a funder
    pub fn drops_for_funder(
        &self, 
        account_id: AccountId,
        from_index: Option<U128>, 
        limit: Option<u64>
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
                .map(|id| self.get_drop_information(id))
                // Collect all JsonDrops into a vector and return it
                .collect()
        } else {
            return vec![];
        }
    }

    /// Returns the current nonce on the contract
    pub fn get_nonce(&self) -> u128 {
        self.nonce
    }
}   