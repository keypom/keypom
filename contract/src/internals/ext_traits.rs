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
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);

    fn storage_balance_bounds(&self) -> StorageBalanceBounds;
}

#[ext_contract(ext_self)]
trait ExtThis {
    /// self callback for simple linkdrops with no FTs, NFTs, or FCs.
    fn on_claim_simple(
        &mut self,
        // Account ID that sent the funds for the linkdrop
        funder_id: AccountId,
        // Balance contained within the linkdrop
        balance: U128,
        // How much storage was used up for the linkdrop
        storage_used: U128,
    ) -> bool;

    /// self callback for FT linkdrop
    fn on_claim_ft(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        funder_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Who sent the FTs?
        ft_sender: AccountId,
        // Where are the FTs stored
        ft_contract: AccountId,
        // How many FTs should we send
        ft_balance: U128,
        // How much storage does it cost to register the new account
        ft_storage: U128,
    ) -> bool;

    /// self callback for a linkdrop loaded with an NFT
    fn on_claim_nft(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        funder_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Sender of the NFT
        nft_sender: AccountId,
        // Contract where the NFT is stored
        nft_contract: AccountId,
        // Token ID for the NFT
        token_id: String,
    ) -> bool;

    /// self callback checks if account was created successfully or not. If yes, refunds excess storage, sends NFTs, FTs etc..
    fn on_claim_fc(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        funder_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Receiver of the function call
        receiver: AccountId,
        // Method to call on the contract
        method: String,
        // What args to pass in
        args: String,
        // What deposit should we attach
        deposit: U128,
        // Should the refund be sent to the funder or attached to the deposit
        add_refund_to_deposit: Option<bool>,
        // Should we add the account ID as part of the args and what key should it live in
        claimed_account_field: Option<String>,
    ) -> bool;

    fn nft_resolve_transfer(
        &mut self,
        token_id: String,
        token_sender: AccountId,
        token_contract: AccountId,
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
