use crate::*;

#[near_bindgen]
impl Keypom {
    /// Allows you to query for the total number of drops currently owned by a given funder
    ///
    ///
    /// Arguments:
    /// * `account_id` The account ID of the funder to query for
    ///
    /// Returns a `u64` representing the number of drops owned by the funder. If the funder does not have any drops, returns `0`
    pub fn get_drop_supply_for_funder(&self, account_id: AccountId) -> u64 {
        self.drop_ids_by_funder.get(&account_id).and_then(|d| Some(d.len())).unwrap_or(0)
    }

    /// Allows you to paginate through all the active drops for a given funder
    ///
    /// Requirements:
    /// * Panics if the drop does not exist.
    ///
    /// Arguments:
    /// * `account_id` The account ID of the funder to query for
    /// * `from_index` where to start paginating from. If not specified, will start from 0 index.
    /// * `limit` how many keys to return. If not specified, will return 50 keys.
    ///
    /// Returns a vector of `ExtDrop` objects representing the information about the drops
    pub fn get_drops_for_funder(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<ExtDrop> {
        // Iterate through each drop ID and push JsonDrop to a vector
        let drop_ids = self.drop_ids_by_funder.get(&account_id);

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
                .map(|id| self.get_drop_information(ExtDropOrPublicKey::DropId(id)))
                // Collect all JsonDrops into a vector and return it
                .collect()
        } else {
            return vec![];
        }
    }
}