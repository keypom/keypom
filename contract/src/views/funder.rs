use crate::*;

#[near_bindgen]
impl Keypom {
    /// Returns the total supply of active drops for a given funder
    pub fn get_drop_supply_for_funder(&self, account_id: AccountId) -> u64 {
        self.drop_ids_by_funder.get(&account_id).and_then(|d| Some(d.len())).unwrap_or(0)
    }

    /// Return a vector of drop information for a funder
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