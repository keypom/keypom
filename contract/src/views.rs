use crate::*;

#[near_bindgen]
impl DropZone {
    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let account_data = self.data_for_pk
            .get(&key)
            .expect("Key missing");
        (account_data.balance.0).into()
    }

    /*
        CUSTOM
    */
    
    /// Query for the total supply of linkdrops on the contract
    pub fn key_total_supply(
        &self
    ) -> U128 {
        //return the length of the data_for_pk set
        U128(self.data_for_pk.len() as u128)
    }

    /// Paginate through all active keys on the contract and return a vector of key info.
    pub fn get_keys(
        &self, 
        from_index: Option<U128>, 
        limit: Option<u64>
    ) -> Vec<KeyInfo> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        self.data_for_pk.keys()
            //skip to the index we specified in the start variable
            .skip(start as usize) 
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize) 
            //we'll map the public keys which are strings into KeyInfos
            .map(|pk| self.get_key_information(pk.clone()))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    
    /// Returns the total supply of active keys for a given funder
    pub fn key_supply_for_funder(
        &self, 
        account_id: AccountId,
    ) -> U128 {
        //get the set of keys for the passed in funder
        let keys_for_owner = self.keys_for_funder.get(&account_id);

        //if there is some set of keys, we'll return the length as a U128
        if let Some(keys_for_owner) = keys_for_owner {
            U128(keys_for_owner.len() as u128)
        } else {
            //if there isn't a set of keys for the passed in account ID, we'll return 0
            U128(0)
        }
    }

    /// Paginate through active keys for a given funder and return the key info.
    pub fn keys_for_funder(
        &self, 
        account_id: AccountId,
        from_index: Option<U128>, 
        limit: Option<u64>
    ) -> Vec<KeyInfo> {
        //get the set of keys for the passed in funder
        let keys_for_owner = self.keys_for_funder.get(&account_id);
        
        //if there is some set of keys, we'll set the public_keys variable equal to that set
        let public_keys = if let Some(keys_for_owner) = keys_for_owner {
            keys_for_owner
        } else {
            //if there is no set of keys, we'll simply return an empty vector. 
            return vec![];
        };

        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through the public keys
        public_keys.iter()
            //skip to the index we specified in the start variable
            .skip(start as usize) 
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize) 
            //we'll map the public keys which are strings into KeyInfos
            .map(|pk| self.get_key_information(pk.clone()))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    /// Returns the data corresponding to a specific key
    pub fn get_key_information(
        &self,
        key: PublicKey
    ) -> KeyInfo {
        // By default, every key should have account data
        let account_data = self.data_for_pk
            .get(&key);

        // If there's no account data, return none across the board.
        if account_data.is_none() {
            return KeyInfo {
                pk: None,
                account_data: None,
                fc_data: None,
                nft_data: None,
                ft_data: None
            }
        }

        // Default all callback data to None
        let mut key_info = KeyInfo {
            pk: Some(key),
            account_data: account_data.clone(),
            fc_data: None,
            nft_data: None,
            ft_data: None
        };

        // If there's a Nonce, return all callback data related to that nonce.
        if let Some(nonce) = account_data.unwrap().cb_id {
            key_info.ft_data = self.ft.get(&nonce);
            key_info.nft_data = self.nft.get(&nonce);
            key_info.fc_data = self.fc.get(&nonce);
        }

        // Return the key info
        key_info
    }

    /// Returns the current nonce on the contract
    pub fn get_nonce(&self) -> u64 {
        self.nonce
    }
}   