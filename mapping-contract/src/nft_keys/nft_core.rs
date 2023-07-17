use crate::*;
#[ext_contract(ext_non_fungible_token_receiver)]
trait NonFungibleTokenReceiver {
    //Method stored on the receiver contract that is called via cross contract call when nft_transfer_call is called
    /// Returns `true` if the token should be returned back to the sender.
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> Promise;
}

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn nft_transfer(
        &mut self,
        token_id: Option<TokenId>,
        receiver_id: Option<AccountId>,
        approval_id: Option<u64>,
        memo: PublicKey,
    ) {
        let sender_id = env::predecessor_account_id();
        let sender_pk = env::signer_account_pk();
        let receiver_id = receiver_id.unwrap_or(env::current_account_id());

        // Token ID is either from sender PK or passed in
        let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or_else(|| token_id.expect("Token ID not provided"));
        self.internal_transfer(sender_id, receiver_id, token_id, approval_id, memo, None);
    }

    //get the information for a specific token ID
    pub fn nft_token(&self, token_id: TokenId) -> Option<ExtNFTKey> {
        let drop_id = parse_token_id(&token_id).0;
        if let Some(drop) = self.drop_by_id.get(&drop_id) {
            let config = drop.nft_config;

            let nft_metadata = config.as_ref().and_then(|c| c.token_metadata.clone());
            let royalty = config.and_then(|c| c.royalties);

            if let Some(key_info) = drop.key_info_by_token_id.get(&token_id) {
                return Some(ExtNFTKey {
                    token_id,
                    owner_id: key_info.owner_id,
                    metadata: nft_metadata.unwrap_or(TokenMetadata {
                        title: Some(String::from("Keypom Access Key")),
                        description: Some(String::from("Keypom is pretty lit")),
                        media: Some(String::from("bafybeibwhlfvlytmttpcofahkukuzh24ckcamklia3vimzd4vkgnydy7nq")),
                        media_hash: None,
                        copies: None,
                        issued_at: None,
                        expires_at: None,
                        starts_at: None,
                        updated_at: None,
                        extra: None,
                        reference: None,
                        reference_hash: None,
                    }),
                    approved_account_ids: key_info.approved_account_ids.clone(),
                    royalty: royalty.unwrap_or_default(),
                })
            }
        }
        
        return None;
    }
}