use crate::*;

#[near_bindgen]
impl Keypom {
    /// Transfers an NFT key from one user to another.
    /// If *token_id* is passed in, we use that token ID. Otherwise, we use the token ID associated with the sender's public key
    /// If *receiver_id* is passed in, we transfer the token to that account. Otherwise, we transfer the token to the current account.
    /// This functionality is added in case you want to transfer to someone who doesn't have a NEAR wallet.
    /// The *memo* field is the new public key that the token will be associated with.
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
        self.internal_transfer(sender_id, receiver_id, token_id, approval_id, memo);
    }

    /// Get the token object info for a specific token ID
    pub fn nft_token(&self, token_id: TokenId) -> Option<ExtNFTKey> {
        let drop_id = parse_token_id(&token_id).0;

        if let Some(drop) = self.drop_by_id.get(&drop_id) {
            let NFTKeyBehaviour { token_metadata, royalties } = drop.nft_config.unwrap_or(NFTKeyBehaviour {
                token_metadata: None,
                royalties: None,
            });

            if let Some(key_info) = drop.key_info_by_token_id.get(&token_id) {
                return Some(ExtNFTKey {
                    token_id,
                    owner_id: key_info.owner_id,
                    metadata: token_metadata.unwrap_or(TokenMetadata {
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
                    royalty: royalties.unwrap_or_default(),
                })
            }
        }
        
        return None;
    }
}