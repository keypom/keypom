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

#[ext_contract(ext_self)]
trait ExtThis {
    fn on_claim(
        &mut self, 
        new_account_id: AccountId, 
        balance: U128, 
        funder_id: AccountId, 
        nft_id: Option<String>, 
        token_contract: Option<AccountId>,
        token_sender: Option<AccountId>
    ) -> bool;

    fn nft_resolve_transfer(
        &mut self,
        token_id: String,
        token_sender: AccountId,
        token_contract: AccountId
    );
}