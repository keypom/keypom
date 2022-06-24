use crate::*;

#[near_bindgen]
impl DropZone {
    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let drop_id = self.drop_id_for_pk.get(&key).expect("no drop ID found for key");
        let drop = self.drop_type_for_id.get(&drop_id).expect("no drop found for drop ID");
        (drop.balance.0).into()
    }

    /*
        CUSTOM
    */
    
    /// Query for the total supply of linkdrops on the contract
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
    ) -> Vec<DropType> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        self.drop_id_for_pk.keys()
            //skip to the index we specified in the start variable
            .skip(start as usize) 
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize) 
            //we'll map the public key which are strings into DropTypes
            .map(|pk| self.get_key_information(pk.clone()))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    
    /// Returns the total supply of active keys for a given funder
    pub fn key_supply_for_funder(
        &self, 
        account_id: AccountId,
    ) -> U128 {
        //get the set of drops for the passed in funder
        let drops_for_owner = self.drops_for_funder.get(&account_id);

        //if there is some set of drops, we'll iterate through and collect all the keys
        if let Some(drops_for_owner) = drops_for_owner {
            let mut supply = 0;
            for id in drops_for_owner.iter() {
                supply += self.drop_type_for_id.get(&id).unwrap().len;
            }

            U128(supply)
        } else {
            //if there isn't a set of keys for the passed in account ID, we'll return 0
            U128(0)
        }
    }

    /// Returns the total supply of active drops for a given funder
    pub fn drop_supply_for_funder(
        &self, 
        account_id: AccountId,
    ) -> U128 {
        //get the set of drops for the passed in funder
        let drops_for_owner = self.drops_for_funder.get(&account_id);

        //if there is some set of drops, we'll return the length
        if let Some(drops_for_owner) = drops_for_owner {
            U128(drops_for_owner.len() as u128)
        } else {
            //if there isn't a set of keys for the passed in account ID, we'll return 0
            U128(0)
        }
    }

    /// Paginate through active keys for a given funder and return the drop info.
    pub fn drops_for_funder(
        &self, 
        account_id: AccountId,
        from_index: Option<U128>, 
        limit: Option<u64>
    ) -> Vec<DropType> {
        //get the set of drops for the passed in funder
        let drops_for_owner = self.drops_for_funder.get(&account_id);
        
        //if there is some set of drops, we'll set the public_keys variable equal to that set
        let drops = if let Some(drops) = drops_for_owner {
            drops
        } else {
            //if there is no set of keys, we'll simply return an empty vector. 
            return vec![];
        };

        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through the public keys
        drops.iter()
            //skip to the index we specified in the start variable
            .skip(start as usize) 
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize) 
            //we'll map the drop ids into KeyInfos
            .map(|drop_id| self.get_drop_information(drop_id.clone()))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    /// Returns the DropType corresponding to a specific key
    pub fn get_key_information(
        &self,
        key: PublicKey
    ) -> DropType {
        let drop_id = self.drop_id_for_pk.get(&key).expect("no drop ID found for key");
        let drop = self.drop_type_for_id.get(&drop_id).expect("no drop found for drop ID");

        drop
    }

    /// Returns the DropType corresponding to a specific key
    pub fn get_drop_information(
        &self,
        drop_id: DropId
    ) -> DropType {
        let drop = self.drop_type_for_id.get(&drop_id).expect("no drop found for drop ID");

        drop
    }

    /// Returns the current nonce on the contract
    pub fn get_nonce(&self) -> u128 {
        self.nonce
    }
}   