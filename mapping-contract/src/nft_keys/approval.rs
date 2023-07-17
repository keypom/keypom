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
    pub fn nft_approve(&mut self, token_id: Option<TokenId>, account_id: AccountId, msg: Option<String>) {
        let sender_id = env::predecessor_account_id();
        let sender_pk = env::signer_account_pk();

        // Token ID is either from sender PK or passed in
        let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or_else(|| token_id.expect("Token ID not provided"));
        let drop_id = parse_token_id(&token_id).0;
        
        // Get drop in order to get key info
        let mut drop = self.drop_for_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");

        if sender_id == env::current_account_id() {
            // Ensure the key has enough allowance
            require!(
                key_info.allowance >= env::prepaid_gas().0 as u128 * self.yocto_per_gas,
                "Not enough allowance on the key."
            );
            
            key_info.allowance -= (env::used_gas().0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
        } else {
            require!(
                key_info.owner_id == sender_id,
                "Sender does not own this token"
            );
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
	pub fn nft_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<u64>,
    ) -> bool {
        //get the key info object from the token_id
        let drop_id = parse_token_id(&token_id).0;
    
        // Get drop in order to get key info
        let drop = self.drop_for_id.get(&drop_id).expect("Drop not found");
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
        let mut drop = self.drop_for_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");

        if sender_id == env::current_account_id() {
            // Ensure the key has enough allowance
            require!(
                key_info.allowance >= env::prepaid_gas().0 as u128 * self.yocto_per_gas,
                "Not enough allowance on the key."
            );
            
            key_info.allowance -= (env::used_gas().0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
        } else {
            require!(
                key_info.owner_id == sender_id,
                "Sender does not own this token"
            );
        }

        //if the account ID was in the token's approval, we remove it and the if statement logic executes
        if key_info
            .approved_account_ids
            .remove(&account_id)
            .is_some()
        {
            // Reinsert key info mapping to NFT and then add token ID mapping to public key
            drop.key_info_by_token_id.insert(&token_id, &key_info);
            self.drop_for_id.insert(&drop_id, &drop);
        }
    }

    //revoke all accounts from transferring the token on your behalf
    #[payable]
    pub fn nft_revoke_all(&mut self, token_id: Option<TokenId>) {
        let sender_id = env::predecessor_account_id();
        let sender_pk = env::signer_account_pk();

        // Token ID is either from sender PK or passed in
        let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or_else(|| token_id.expect("Token ID not provided"));
        let drop_id = parse_token_id(&token_id).0;
        
        // Get drop in order to get key info
        let mut drop = self.drop_for_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop.key_info_by_token_id.get(&token_id).expect("Key info not found");

        if sender_id == env::current_account_id() {
            // Ensure the key has enough allowance
            require!(
                key_info.allowance >= env::prepaid_gas().0 as u128 * self.yocto_per_gas,
                "Not enough allowance on the key."
            );
            
            key_info.allowance -= (env::used_gas().0 + GAS_FOR_PANIC_OFFSET.0) as u128 * self.yocto_per_gas;
        } else {
            require!(
                key_info.owner_id == sender_id,
                "Sender does not own this token"
            );
        }

        //if the account ID was in the token's approval, we remove it and the if statement logic executes
        key_info
            .approved_account_ids
            .clear();

        // Reinsert key info mapping to NFT and then add token ID mapping to public key
        drop.key_info_by_token_id.insert(&token_id, &key_info);
        self.drop_for_id.insert(&drop_id, &drop);
    }
}