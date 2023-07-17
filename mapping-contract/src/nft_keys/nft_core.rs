use crate::*;
use near_sdk::{ext_contract};

// const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(10_000_000_000_000);
// const GAS_FOR_NFT_ON_TRANSFER: Gas = Gas(25_000_000_000_000);

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

    // //implementation of the transfer call method. This will transfer the NFT and call a method on the receiver_id contract
    // #[payable]
    // pub fn nft_transfer_call(
    //     &mut self,
    //     receiver_id: AccountId,
    //     token_id: Option<TokenId>,
    //     //we introduce an approval ID so that people with that approval ID can transfer the token
    //     approval_id: Option<u64>,
    //     memo: PublicKey,
    //     msg: String,
    // ) -> PromiseOrValue<bool> {
    //     let sender_id = env::predecessor_account_id();
    //     let sender_pk = env::signer_account_pk();

    //     // Token ID is either from sender PK or passed in
    //     let token_id = self.token_id_by_pk.get(&sender_pk).unwrap_or_else(|| token_id.expect("Token ID not provided"));
    //     let old_key_info = self.internal_transfer(sender_id, receiver_id, token_id, approval_id, memo, Some(GAS_FOR_NFT_ON_TRANSFER + GAS_FOR_RESOLVE_TRANSFER));
        
    //     //default the authorized_id to none
    //     let mut authorized_id = None; 
    //     //if the sender isn't the owner of the token, we set the authorized ID equal to the sender.
    //     if sender_id != old_key_info.owner_id {
    //         authorized_id = Some(sender_id.to_string());
    //     }

    //     // Initiating receiver's call and the callback
    //     // Defaulting GAS weight to 1, no attached deposit, and static GAS equal to the GAS for nft on transfer.
    //     ext_non_fungible_token_receiver::ext(receiver_id.clone())
    //         .with_static_gas(GAS_FOR_NFT_ON_TRANSFER)
    //         .nft_on_transfer(
    //             sender_id, 
    //             old_key_info.owner_id.clone(), 
    //             token_id.clone(), 
    //             msg
    //         )
    //     // We then resolve the promise and call nft_resolve_transfer on our own contract
    //     .then(
    //         // Defaulting GAS weight to 1, no attached deposit, and static GAS equal to the GAS for resolve transfer
    //         Self::ext(env::current_account_id())
    //             .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
    //             .nft_resolve_transfer(
    //                 authorized_id, // we introduce an authorized ID so that we can log the transfer
    //                 previous_token.owner_id,
    //                 receiver_id,
    //                 token_id,
    //                 previous_token.approved_account_ids,
    //                 memo, // we introduce a memo for logging in the events standard
    //             )
    //     ).into()
    // }

    //get the information for a specific token ID
    pub fn nft_token(&self, token_id: TokenId) -> Option<JsonToken> {
        let drop_id = parse_token_id(&token_id).0;
        if let Some(drop) = self.drop_for_id.get(&drop_id) {
            let config = drop.config;

            let nft_metadata = config.as_ref().and_then(|c| c.nft_key_behaviour.clone()).and_then(|b| b.nft_metadata.clone());
            let royalty = config.and_then(|c| c.nft_key_behaviour).and_then(|b| b.nft_royalty);

            if let Some(key_info) = drop.key_info_by_token_id.get(&token_id) {
                return Some(JsonToken {
                    token_id,
                    owner_id: key_info.owner_id,
                    metadata: nft_metadata.unwrap_or(TokenMetadata {
                        title: Some(String::from("Keypom Access Key")),
                        description: Some(String::from("Keypom is pretty lit")),
                        media: Some(String::from("Keypom is pretty lit")),
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