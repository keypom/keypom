use crate::*;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct NftApproveMsg {
    pub linkdrop_pk: PublicKey,
    pub signature: Base64VecU8,
    pub msg: Option<String>,
}

#[near_bindgen]
impl Keypom {
    /// Allow a specific account ID to transfer a token on your behalf
    #[payable]
    pub fn nft_approve(&mut self, account_id: AccountId, msg: String) {
        self.assert_no_global_freeze();
        // Deserialize the msg string into the NftApproveMsg struct
        let nft_approve_msg: NftApproveMsg =
            serde_json::from_str(&msg).expect("Invalid message format");
        let NftApproveMsg {
            linkdrop_pk,
            signature,
            msg: msg_str,
        } = nft_approve_msg;

        let args_string = json!({
            "account_id": account_id,
            "msg": msg
        }).to_string();
    
        require!(
            self.verify_signature(signature, linkdrop_pk.clone(), args_string),
            "Invalid signature for public key"
        );


        let sender_id = env::predecessor_account_id();

        // Token ID is either from sender PK or passed in
        let token_id = self
            .token_id_by_pk
            .get(&linkdrop_pk)
            .expect("Token ID not found");
        let drop_id = parse_token_id(&token_id).unwrap().0;

        // Get drop in order to get key info
        let mut drop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let mut key_info = drop
            .key_info_by_token_id
            .get(&token_id)
            .expect("Key info not found");

        // Check that if the drop config has a resale set, the approval ID is in that set
        if let Some(resale_allowlist) = drop
            .config
            .as_ref()
            .and_then(|c| c.transfer_key_allowlist.as_ref())
        {
            require!(
                resale_allowlist.contains(&account_id),
                "Approval ID not in resale allowlist"
            );
        }

        // Check that the sender is the owner of the token.
        // If the token is owned by keypom, decrement the key's allowance
        check_key_owner(sender_id, &key_info);

        // Get the next approval ID if we need a new approval
        let approval_id: u64 = key_info.next_approval_id;
        key_info
            .approved_account_ids
            .insert(account_id.clone(), approval_id);
        key_info.next_approval_id += 1;

        // Reinsert key info mapping to NFT and then add token ID mapping to public key
        drop.key_info_by_token_id.insert(&token_id, &key_info);
        self.drop_by_id.insert(&drop_id, &drop);

        //if some message was passed into the function, we initiate a cross contract call on the
        //account we're giving access to.
        if let Some(msg) = msg_str {
            // Defaulting GAS weight to 1, no attached deposit, and no static GAS to attach.
            Promise::new(account_id)
                .function_call_weight(
                    "nft_on_approve".to_string(),
                    json!({ "token_id": token_id, "owner_id": key_info.owner_id.unwrap_or(env::current_account_id()), "approval_id": approval_id, "msg": msg }).to_string().into(),
                    NearToken::from_yoctonear(0),
                    Gas::from_gas(0),
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
        let drop_id = parse_token_id(&token_id).unwrap().0;

        // Get drop in order to get key info
        let drop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop
            .key_info_by_token_id
            .get(&token_id)
            .expect("Key info not found");

        //get the approval number for the passed in account ID
        let approval = key_info.approved_account_ids.get(&approved_account_id);

        //if there was some approval ID found for the account ID
        if let Some(approval) = approval {
            //if a specific approval_id was passed into the function
            if let Some(approval_id) = approval_id {
                //return if the approval ID passed in matches the actual approval ID for the account
                approval_id == *approval
                //if there was no approval_id passed into the function, we simply return true
            } else {
                true
            }
            //if there was no approval ID found for the account ID, we simply return false
        } else {
            false
        }
    }
}

/// Check that the sender is either the owner of the token or the current account (meaning they signed with the key).
pub(crate) fn check_key_owner(sender_id: AccountId, key_info: &InternalKeyInfo) {
    if sender_id != env::current_account_id() {
        require!(
            key_info
                .owner_id
                .as_ref()
                .unwrap_or(&env::current_account_id())
                == &sender_id,
            "Sender does not own this token"
        );
    }
}
