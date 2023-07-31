use crate::*;

#[near_bindgen]
impl Keypom {
    pub(crate) fn parse_key_or_token_id(&self, key_or_token_id: ExtKeyOrTokenId) -> String {
        match key_or_token_id {
            ExtKeyOrTokenId::TokenId(token_id) => token_id,
            ExtKeyOrTokenId::PublicKey(key) => self
                .token_id_by_pk
                .get(&key)
                .expect("Token ID not found for Public Key"),
        }
    }

    pub(crate) fn parse_drop_or_pk(&self, drop_or_pk: ExtDropOrPublicKey) -> DropId {
        match drop_or_pk {
            ExtDropOrPublicKey::DropId(drop_id) => drop_id,
            ExtDropOrPublicKey::PublicKey(key) => parse_token_id(&self
                .token_id_by_pk
                .get(&key)
                .expect("Token ID not found for Public Key")).0,
        }
    }
}