use crate::*;
use near_sdk::{ext_contract};

#[ext_contract(ext_non_fungible_approval_receiver)]
trait NonFungibleTokenApprovalsReceiver {
    //cross contract call to an external contract that is initiated during nft_approve
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    );
}

#[near_bindgen]
impl Keypom {

    /// Allow a specific account ID to transfer a token on your behalf
    #[payable]
    fn nft_approve(&mut self, token_id: Option<TokenId>, account_id: AccountId, msg: Option<String>) {
        let sender_id = env::predecessor_account_id();
        let sender_pk = env::signer_account_pk();

        // Token ID is either from sender PK or passed in
        let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or_else(|| token_id.expect("Token ID not provided"));
        let drop_id = parse_token_id(&token_id).0;
        
        // Get drop in order to get key info
        let mut drop = self.drop_for_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");
        
        require!(
            key_info.owner_id == sender_id,
            "Sender does not own this token"
        );

        if sender_id == env::current_account_id() {
            // Ensure the key has enough allowance
            require!(
                key_info.allowance >= env::prepaid_gas().0 as u128 * self.yocto_per_gas,
                "Not enough allowance on the key."
            );
            
            key_info.allowance -= (env::used_gas().0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
        }

        //get the next approval ID if we need a new approval
        let approval_id: u64 = key_info.next_approval_id;
        key_info.approved_account_ids.insert(account_id.clone(), approval_id);
        key_info.next_approval_id += 1;

        // Reinsert key info mapping to NFT and then add token ID mapping to public key
        drop.key_info_by_token_id.insert(&token_id, &key_info);
        self.drop_for_id.insert(&drop_id, &drop);
        
        //if some message was passed into the function, we initiate a cross contract call on the
        //account we're giving access to. 
        if let Some(msg) = msg {
            // Defaulting GAS weight to 1, no attached deposit, and no static GAS to attach.
            ext_non_fungible_approval_receiver::ext(account_id)
                .nft_on_approve(
                    token_id, 
                    key_info.owner_id, 
                    approval_id, 
                    msg
                ).as_return();
        }
    }

    //check if the passed in account has access to approve the token ID
	fn nft_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<u64>,
    ) -> bool {
        // //get the token object from the token_id
        // let token = self.tokens_by_id.get(&token_id).expect("No token");

        // //get the approval number for the passed in account ID
		// let approval = token.approved_account_ids.get(&approved_account_id);

        // //if there was some approval ID found for the account ID
        // if let Some(approval) = approval {
        //     //if a specific approval_id was passed into the function
		// 	if let Some(approval_id) = approval_id {
        //         //return if the approval ID passed in matches the actual approval ID for the account
		// 		approval_id == *approval
        //     //if there was no approval_id passed into the function, we simply return true
		// 	} else {
		// 		true
		// 	}
        // //if there was no approval ID found for the account ID, we simply return false
		// } else {
		// 	false
		// }
        false
    }

    //revoke a specific account from transferring the token on your behalf 
    #[payable]
    fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId) {
        // //assert that the user attached exactly 1 yoctoNEAR for security reasons
        // assert_one_yocto();
        // //get the token object using the passed in token_id
        // let mut token = self.tokens_by_id.get(&token_id).expect("No token");

        // //get the caller of the function and assert that they are the owner of the token
        // let predecessor_account_id = env::predecessor_account_id();
        // assert_eq!(&predecessor_account_id, &token.owner_id);

        // //if the account ID was in the token's approval, we remove it and the if statement logic executes
        // if token
        //     .approved_account_ids
        //     .remove(&account_id)
        //     .is_some()
        // {
        //     //refund the funds released by removing the approved_account_id to the caller of the function
        //     refund_approved_account_ids_iter(predecessor_account_id, [account_id].iter());

        //     //insert the token back into the tokens_by_id collection with the account_id removed from the approval list
        //     self.tokens_by_id.insert(&token_id, &token);
        // }
    }

    //revoke all accounts from transferring the token on your behalf
    #[payable]
    fn nft_revoke_all(&mut self, token_id: TokenId) {
        // //assert that the caller attached exactly 1 yoctoNEAR for security
        // assert_one_yocto();

        // //get the token object from the passed in token ID
        // let mut token = self.tokens_by_id.get(&token_id).expect("No token");
        // //get the caller and make sure they are the owner of the tokens
        // let predecessor_account_id = env::predecessor_account_id();
        // assert_eq!(&predecessor_account_id, &token.owner_id);

        // //only revoke if the approved account IDs for the token is not empty
        // if !token.approved_account_ids.is_empty() {
        //     //refund the approved account IDs to the caller of the function
        //     refund_approved_account_ids(predecessor_account_id, &token.approved_account_ids);
        //     //clear the approved account IDs
        //     token.approved_account_ids.clear();
        //     //insert the token back into the tokens_by_id collection with the approved account IDs cleared
        //     self.tokens_by_id.insert(&token_id, &token);
        // }
    }
}