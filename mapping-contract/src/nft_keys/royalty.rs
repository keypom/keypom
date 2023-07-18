use std::collections::HashMap;

use crate::*;

/// The maximum number of people that can be paid out to in a single transaction
const MAX_LEN_PAYOUT: u32 = 10;

#[near_bindgen]
impl Keypom {
    /// Transfers an NFT key from one user to another and returns the payout object that should be payed given the passed in balance.
    /// If *token_id* is passed in, we use that token ID. Otherwise, we use the token ID associated with the sender's public key
    /// If *receiver_id* is passed in, we transfer the token to that account. Otherwise, we transfer the token to the current account.
    /// This functionality is added in case you want to transfer to someone who doesn't have a NEAR wallet.
    /// The *memo* field is the new public key that the token will be associated with.
    #[payable]
    pub fn nft_transfer_payout(
        &mut self,
        receiver_id: Option<AccountId>,
        token_id: Option<TokenId>,
        approval_id: Option<u64>,
        memo: PublicKey,
        balance: U128,
        max_len_payout: Option<u32>,
    ) -> Payout {
        let sender_id = env::predecessor_account_id();
        let sender_pk = env::signer_account_pk();
        // Receiver is either passed in or current account
        let receiver_id = receiver_id.unwrap_or(env::current_account_id());
        // Token ID is either from sender PK or passed in
        let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or_else(|| token_id.expect("Token ID not provided"));
        
        // Get drop in order to get key info and royalties
        let drop_id = parse_token_id(&token_id).0;
        let drop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        
        // Get royalties from key info
        let default_royalty = &HashMap::new();
        let nft_royalty = drop.nft_config.as_ref().and_then(|c| c.royalties.as_ref()).unwrap_or(default_royalty);

        // Perform the transfer and then calculate payouts
        let old_owner_id = self.internal_transfer(sender_id, receiver_id, token_id, approval_id, memo, None);
        
        calculate_payouts(old_owner_id, nft_royalty.clone(), u128::from(balance), max_len_payout.unwrap_or(MAX_LEN_PAYOUT))
    }

    /// Calculates the payout for a token given the passed in balance.
    pub fn nft_payout(&self, token_id: TokenId, balance: U128, max_len_payout: Option<u32>) -> Payout {
        //get the key info object from the token_id
        let drop_id = parse_token_id(&token_id).0;
    
        let drop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let default_royalty = &HashMap::new();
        let nft_royalty = drop.nft_config.as_ref().and_then(|c| c.royalties.as_ref()).unwrap_or(default_royalty);
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");

        calculate_payouts(key_info.owner_id, nft_royalty.clone(), u128::from(balance), max_len_payout.unwrap_or(MAX_LEN_PAYOUT))
	}
}

/// Helper function to convert a royalty percentage and amount to a payout
pub(crate) fn calculate_payouts(owner_id: AccountId, royalties: HashMap<AccountId, u32>, balance: u128, max_len_payout: u32) -> Payout {        
        //keep track of the total perpetual royalties
        let mut total_perpetual = 0;
        //keep track of the payout object to send back
        let mut payout_object = Payout {
            payout: HashMap::new(),
        };

        //make sure we're not paying out to too many people (GAS limits this)
        assert!(
            royalties.len() as u32 <= max_len_payout,
            "Market cannot payout to that many receivers"
        );

        //go through each key and value in the royalty object
        for (k, v) in royalties.iter() {
            //get the key
            let key = k.clone();
            //only insert into the payout if the key isn't the token owner (we add their payout at the end)
            if key != owner_id {
                //
                payout_object
                    .payout
                    .insert(key, royalty_to_payout(*v, balance));
                total_perpetual += *v;
            }
        }

        // payout to previous owner who gets 100% - total perpetual royalties
        payout_object.payout.insert(
            owner_id,
            royalty_to_payout(10000 - total_perpetual, balance),
        );

        //return the payout object
        payout_object
}

// Convert the royalty percentage and amount to pay into a payout (U128)
pub(crate) fn royalty_to_payout(royalty_percentage: u32, amount_to_pay: Balance) -> U128 {
    U128(royalty_percentage as u128 * amount_to_pay / 10_000u128)
}