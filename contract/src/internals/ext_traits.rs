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

    fn storage_deposit(&mut self, account_id: Option<AccountId>, registration_only: Option<bool>);

    fn storage_balance_bounds(&self) -> StorageBalanceBounds;
}

#[ext_contract(ext_self)]
trait ExtThis {
    /// self callback for simple linkdrops with no FTs, NFTs, or FCs.
    fn on_claim_simple(
        &mut self,
        // Account ID that sent the funds for the linkdrop
        owner_id: AccountId,
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
        owner_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Who sent the FTs?
        sender_id: AccountId,
        // Where are the FTs stored
        contract_id: AccountId,
        // How many FTs should we send
        balance_per_use: U128,
        // How much storage does it cost to register the new account
        ft_storage: U128,
    ) -> bool;

    /// self callback for a linkdrop loaded with an NFT
    fn on_claim_nft(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        owner_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Sender of the NFT
        sender_id: AccountId,
        // Contract where the NFT is stored
        contract_id: AccountId,
        // Token ID for the NFT
        token_id: String,
    ) -> bool;

    /// self callback checks if account was created successfully or not. If yes, refunds excess storage, sends NFTs, FTs etc..
    fn on_claim_fc(
        &mut self,
        // Account ID that claimed the linkdrop
        account_id: AccountId,
        // Account ID that funded the linkdrop
        owner_id: AccountId,
        // Balance associated with the linkdrop
        balance: U128,
        // How much storage was used to store linkdrop info
        storage_used: U128,
        // Receiver of the function call
        receiver_id: AccountId,
        // Method to call on the contract
        method_name: String,
        // What args to pass in
        args: String,
        // What attached_deposit should we attach
        attached_deposit: U128,
        // Should the refund be sent to the funder or attached to the attached_deposit
        add_refund_to_deposit: Option<bool>,
        // Should we add the account ID as part of the args and what key should it live in
        account_id_field: Option<String>,
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
        owner_id: AccountId,
        balance: U128,
        required_storage: U128,
        cb_ids: Vec<u64>,
    );
}
