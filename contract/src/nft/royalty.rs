use std::collections::HashMap;

use crate::*;

//convert the royalty percentage and amount to pay into a payout (U128)
pub(crate) fn royalty_to_payout(royalty_percentage: u32, amount_to_pay: Balance) -> U128 {
    U128(royalty_percentage as u128 * amount_to_pay / 10_000u128)
}

#[near_bindgen]
impl Keypom {
    //transfers the token to the receiver ID and returns the payout object that should be payed given the passed in balance.
    #[payable]
    pub fn nft_transfer_payout(
        &mut self,
        receiver_id: Option<AccountId>,
        token_id: Option<TokenId>,
        approval_id: Option<u64>,
        memo: PublicKey,
        balance: U128,
        max_len_payout: u32,
    ) -> Payout {
        let (old_owner_id, nft_royalty) =
            self.nft_transfer(token_id, receiver_id, approval_id, memo);

        //keep track of the total perpetual royalties
        let mut total_perpetual = 0;
        //get the u128 version of the passed in balance (which was U128 before)
        let balance_u128 = u128::from(balance);
        //keep track of the payout object to send back
        let mut payout_object = Payout {
            payout: HashMap::new(),
        };

        //make sure we're not paying out to too many people (GAS limits this)
        assert!(
            nft_royalty.len() as u32 <= max_len_payout,
            "Market cannot payout to that many receivers"
        );

        //go through each key and value in the royalty object
        for (k, v) in nft_royalty.iter() {
            //get the key
            let key = k.clone();
            //only insert into the payout if the key isn't the token owner (we add their payout at the end)
            if key != old_owner_id {
                //
                payout_object
                    .payout
                    .insert(key, royalty_to_payout(*v, balance_u128));
                total_perpetual += *v;
            }
        }

        // payout to previous owner who gets 100% - total perpetual royalties
        payout_object.payout.insert(
            old_owner_id,
            royalty_to_payout(10000 - total_perpetual, balance_u128),
        );

        //return the payout object
        payout_object
    }
}
