use crate::*;

/// external and self callbacks
#[ext_contract(ext_linkdrop)]
trait ExtLinkdrop {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}
#[ext_contract(ext_self)]
trait ExtThis {
    fn on_claim(
        &mut self, 
        new_account_id: AccountId, 
        pk: PublicKey, 
        balance: U128, 
        funder_id: AccountId, 
        nft_id: Option<String>, 
        token_contract: Option<AccountId>
    ) -> bool;
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