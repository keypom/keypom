use crate::*;

#[near_bindgen]
impl Keypom {
    /// Query for the total supply of NFTs on the contract
    pub fn nft_total_supply(&self) -> U128 {
        // Return the length of the token metadata by ID
        U128(self.token_id_by_pk.len() as u128)
    }

    /// Query for nft tokens on the contract regardless of the owner using pagination
    pub fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<ExtNFTKey> {
        // Where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        // Iterate through each token using an iterator
        self.token_id_by_pk.iter()
            // Skip to the index we specified in the start variable
            .skip(start as usize) 
            // Take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize) 
            // We'll map the token IDs which are strings into Json Tokens
            .map(|(_, token_id)| self.nft_token(token_id).unwrap())
            // Since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    /// Get the total supply of NFTs for a given owner
    pub fn nft_supply_for_owner(
        &self,
        account_id: AccountId,
    ) -> U128 {
        // Get the set of tokens for the passed in owner
        let tokens_for_owner_set = self.tokens_per_owner.get(&account_id);

        // If there is some set of tokens, we'll return the length as a U128
        if let Some(tokens_for_owner_set) = tokens_for_owner_set {
            U128(tokens_for_owner_set.len() as u128)
        } else {
            // If there isn't a set of tokens for the passed in account ID, we'll return 0
            U128(0)
        }
    }

    /// Query for all the tokens for an owner
    pub fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<ExtNFTKey> {
        // Get the set of tokens for the passed in owner
        let tokens_for_owner_set = self.tokens_per_owner.get(&account_id);
        // If there is some set of tokens, we'll set the tokens variable equal to that set
        let tokens = if let Some(tokens_for_owner_set) = tokens_for_owner_set {
            tokens_for_owner_set
        } else {
            // If there is no set of tokens, we'll simply return an empty vector. 
            return vec![];
        };

        // Where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        // Iterate through the keys vector
        tokens.iter()
            // Skip to the index we specified in the start variable
            .skip(start as usize) 
            // Take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize) 
            // We'll map the token IDs which are strings into Json Tokens
            .map(|token_id| self.nft_token(token_id.clone()).unwrap())
            // Since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }
}