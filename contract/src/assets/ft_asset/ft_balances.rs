use crate::*;

impl InternalFTData {
    /// Add to the available balance. This should only ever be invoked in:
    /// * `ft_on_transfer` (when the transfer is successful).
    /// * `ft_resolve_refund` (when the refund failed).
    /// * `on_assets_claimed` (when the FT claim failed and we need to refund).
    pub fn add_to_balance_avail(&mut self, amount: &Balance) {
        self.balance_avail += amount;
    }
}

#[near_bindgen]
impl Keypom {
    /// Standard function for accepting FTs to then be claimable as part of linkdrops.
    pub fn ft_on_transfer(
        &mut self,
        amount: U128,
        msg: DropId,
    ) -> PromiseOrValue<U128> {
        let drop_id = msg;
        let asset_id = env::predecessor_account_id();
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");

        let mut asset: InternalAsset = drop.asset_by_id.get(&asset_id.to_string()).expect("Asset not found");
        // Ensure asset is fungible token and then call the internal function
        if let InternalAsset::ft(ft_data) = &mut asset {
            ft_data.add_to_balance_avail(&amount.0);
            near_sdk::log!("Added {} FTs to drop ID {}. New asset amount: {}", amount.0, drop_id, ft_data.balance_avail);
        };

        drop.asset_by_id.insert(&asset_id.to_string(), &asset);

        self.drop_by_id.insert(&drop_id, &drop);

        PromiseOrValue::Value(U128(0))
    }
}