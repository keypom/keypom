use crate::*;

#[near_bindgen]
impl Keypom {
    /// Allows you to query for the information about a specific drop
    ///
    /// Requirements:
    /// * Panics if the key does not exist.
    ///
    /// Arguments:
    /// * `id` either the ID for the drop as a string or a public key currently part of the drop.
    ///
    /// Returns the `ExtDrop` drop information associated with given drop ID.
    pub fn get_drop_information(&self, id: ExtDropOrPublicKey) -> ExtDrop {
        let drop_id = self.parse_drop_or_pk(id);
        let drop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        return drop.to_external_drop();
    }

    /// Returns the total supply of active keys for a given drop
    pub fn get_key_supply_for_drop(&self, id: ExtDropOrPublicKey) -> u64 {
        let drop_id = self.parse_drop_or_pk(id);

        // Get the drop object and return the length
        self.drop_by_id
            .get(&drop_id)
            .expect("no drop found")
            .key_info_by_token_id
            .len()
    }

    /// Paginate through keys in a specific drop
    #[handle_result]
    pub fn get_keys_for_drop(
        &self,
        id: ExtDropOrPublicKey,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Result<Vec<ExtKeyInfo>, String> {
        let drop_id = self.parse_drop_or_pk(id);

        //iterate through each key using an iterator
        let drop = self.drop_by_id
            .get(&drop_id)
            .expect("No drop for given ID");

        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));
        
        return drop
            .key_info_by_token_id
            .keys()
            //skip to the index we specified in the start variable
            .skip(start as usize)
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize)
            //we'll map the public key which are strings into Drops
            .map(|token_id| self.get_key_information(ExtKeyOrTokenId::TokenId(token_id)))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }
}