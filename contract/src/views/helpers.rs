use crate::*;

#[near_bindgen]
impl Keypom {
    /// Returns how many fees the contract has collected
    pub fn get_fees_collected(&self) -> U128 {
        U128(self.fees_collected)
    }

    /// Returns the current linkdrop contract
    pub fn get_root_account(&self) -> String {
        self.root_account.to_string()
    }

    /// Returns the current fees associated with an account
    pub fn get_fees_per_user(&self, account_id: AccountId) -> Option<KeypomFees> {
        self.fees_per_user
            .get(&account_id)
    }

    /// Returns the current contract source metadata
    pub fn contract_source_metadata(&self) -> ContractSourceMetadata {
        self.contract_metadata.get().unwrap()
    }


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