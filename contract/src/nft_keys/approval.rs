use crate::*;

#[near_bindgen]
impl Keypom {
    /// Allow a specific account ID to transfer a token on your behalf
    #[payable]
    pub fn nft_approve(&mut self, token_id: Option<TokenId>, account_id: AccountId, msg: Option<String>) {
        let sender_id = env::predecessor_account_id();
        let sender_pk = env::signer_account_pk();

        // Token ID is either from sender PK or passed in
        let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or_else(|| token_id.expect("Token ID not provided"));
        let drop_id = parse_token_id(&token_id).0;
        
        // Get drop in order to get key info
        let mut drop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");

        // Check that the sender is the owner of the token.
        // If the token is owned by keypom, decrement the key's allowance
        check_key_owner(sender_id, &key_info);

        //get the next approval ID if we need a new approval
        let approval_id: u64 = key_info.next_approval_id;
        key_info.approved_account_ids.insert(account_id.clone(), approval_id);
        key_info.next_approval_id += 1;

        // Reinsert key info mapping to NFT and then add token ID mapping to public key
        drop.key_info_by_token_id.insert(&token_id, &key_info);
        self.drop_by_id.insert(&drop_id, &drop);
        
        //if some message was passed into the function, we initiate a cross contract call on the
        //account we're giving access to. 
        if let Some(msg) = msg {
            // Defaulting GAS weight to 1, no attached deposit, and no static GAS to attach.
            Promise::new(account_id)
                .function_call_weight(
                    "nft_on_approve".to_string(),
                    json!({ "token_id": token_id, "owner_id": key_info.owner_id, "approval_id": approval_id, "msg": msg }).to_string().into(),
                    0,
                    Gas(0),
                    GasWeight(1),
                ).as_return();
        }
    }

    //check if the passed in account has access to approve the token ID
	pub fn nft_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<u64>,
    ) -> bool {
        //get the key info object from the token_id
        let drop_id = parse_token_id(&token_id).0;
    
        // Get drop in order to get key info
        let drop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");
        
        //get the approval number for the passed in account ID
		let approval = key_info.approved_account_ids.get(&approved_account_id);

        //if there was some approval ID found for the account ID
        if let Some(approval) = approval {
            //if a specific approval_id was passed into the function
			if let Some(approval_id) = approval_id {
                //return if the approval ID passed in matches the actual approval ID for the account
				return approval_id == *approval;
            //if there was no approval_id passed into the function, we simply return true
			} else {
				return true;
			}
        //if there was no approval ID found for the account ID, we simply return false
		} else {
			return false;
		}
    }

    //revoke a specific account from transferring the token on your behalf 
    #[payable]
    pub fn nft_revoke(&mut self, token_id: Option<TokenId>, account_id: AccountId) {
        let sender_id = env::predecessor_account_id();
        let sender_pk = env::signer_account_pk();

        // Token ID is either from sender PK or passed in
        let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or_else(|| token_id.expect("Token ID not provided"));
        let drop_id = parse_token_id(&token_id).0;
        
        // Get drop in order to get key info
        let mut drop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");

        // Check that the sender is the owner of the token.
        // If the token is owned by keypom, decrement the key's allowance
        check_key_owner(sender_id, &key_info);

        //if the account ID was in the token's approval, we remove it and the if statement logic executes
        if key_info
            .approved_account_ids
            .remove(&account_id)
            .is_some()
        {
            // Reinsert key info mapping to NFT and then add token ID mapping to public key
            drop.key_info_by_token_id.insert(&token_id, &key_info);
            self.drop_by_id.insert(&drop_id, &drop);
        }
    }

}

/// Check that the sender is either the owner of the token or the current account (meaning they signed with the key).
pub(crate) fn check_key_owner(sender_id: AccountId, key_info: &InternalKeyInfo) {
    if sender_id != env::current_account_id() {
        require!(
            key_info.owner_id == sender_id,
            "Sender does not own this token"
        );
    }
}