use near_sdk::json_types::Base64VecU8;

use crate::*;

/// external and self callbacks
#[ext_contract(ext_linkdrop)]
trait ExtLinkdrop {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}

/// NFT contract
#[ext_contract(ext_nft_contract)]
trait ExtNFTContract {
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        approval_id: Option<u64>,
        memo: Option<String>,
    );
}

/// FT contract
#[ext_contract(ext_ft_contract)]
trait ExtFTContract {
    fn ft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );

    fn storage_balance_bounds(
        &self,
    ) -> StorageBalanceBounds;
}

#[ext_contract(ext_self)]
trait ExtThis {
    fn on_claim(
        &mut self, 
        account_id: AccountId, 
        funder_id: AccountId, 
        balance: U128, 
        storage_used: U128,
        cb_data_sent: bool,
        /*
            EXTRA
        */
        cb_sender: Option<AccountId>,
        cb_contract: Option<AccountId>,
        nft_id: Option<String>, 
        ft_balance: Option<U128>,
        ft_storage: Option<U128>,
        cb_method: Option<String>,
        cb_args: Option<Base64VecU8>,
        cb_deposit: Option<U128>,
    ) -> bool;

    fn nft_resolve_transfer(
        &mut self,
        token_id: String,
        token_sender: AccountId,
        token_contract: AccountId
    );

    fn resolve_storage_check(
        &mut self,
        public_keys: Vec<PublicKey>,
        funder_id: AccountId,
        balance: U128,
        required_storage: U128,
        cb_ids: Vec<u64>,
    );
}