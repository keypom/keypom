use crate::*;

#[near_bindgen]
impl Keypom {
    /// Query for the global secret key info to sign transactions with
    pub fn get_global_secret_key(&self) -> String {
        self.signing_sk.to_string()
    }

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
        self.fees_per_user.get(&account_id)
    }

    pub(crate) fn parse_key_or_token_id(&self, key_or_token_id: String) -> String {
        let is_token_id = parse_token_id(&key_or_token_id).is_ok();

        if is_token_id {
            return key_or_token_id;
        }

        self.token_id_by_pk
            .get(&key_or_token_id.parse().unwrap())
            .expect("Token ID not found for Public Key")
    }
}
