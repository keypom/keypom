use crate::*;

impl InternalNFTData {
    /// Add to the list of available token IDs that can be claimed. 
    /// This should only ever be invoked in:
    /// * `nft_on_transfer` (when the transfer is successful).
    /// * `nft_resolve_refund` (when the refund failed).
    /// * `on_assets_claimed` (when the NFT claim failed and we need to refund).
    pub fn add_to_token_ids(&mut self, token_id: &TokenId) {
        self.token_ids.push(token_id.clone());
    }
}

#[near_bindgen]
impl Keypom {
    /// Standard function for accepting NFTs to then be claimable as part of linkdrops.
    pub fn nft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        msg: DropId,
    ) -> PromiseOrValue<bool> {
        let initial_storage = env::storage_usage();
        let drop_id = msg;
        let asset_id = env::predecessor_account_id();
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");

        // For NFTs, we should assert here so there's not a malicious attack where someone sends really long NFTs
        // and then the funder has to pay for the storage.
        require!(drop.funder_id == sender_id, "Only the funder can add NFTs to the drop");

        let mut asset: InternalAsset = drop.asset_by_id.get(&asset_id.to_string()).expect("Asset not found");
        // Ensure asset is an NFT and then call the internal function
        if let InternalAsset::nft(nft_data) = &mut asset {
            nft_data.add_to_token_ids(&token_id);
            near_sdk::log!("Added Token ID: {} to drop ID {}. There are now {} NFTs available for claim", token_id, drop_id, nft_data.token_ids.len() as u32);
        };

        drop.asset_by_id.insert(&asset_id.to_string(), &asset);

        self.drop_by_id.insert(&drop_id, &drop);

        // Charge the user for storing the token IDs
        let final_storage = env::storage_usage();
        let total_cost = (final_storage - initial_storage) as u128 * env::storage_byte_cost();
        self.internal_modify_user_balance(&drop.funder_id, total_cost as u128, true);

        PromiseOrValue::Value(false)
    }
}