use crate::*;

#[near_bindgen]
impl Keypom {
    #[payable]
    pub fn claim(&mut self, receiver_id: AccountId) {
        let signer_pk = env::signer_account_pk();
        let drop_id = self.drop_id_for_pk.get(&signer_pk).expect("Drop not found");

        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_pk.remove(&signer_pk).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let KeyBehavior {assets_metadata, config: _} = drop.key_behavior_by_use.remove(&cur_key_use).expect("Use number not found");

        for metadata in assets_metadata {
            let mut asset: InternalAsset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            asset.claim_asset(&drop_id, &receiver_id, &metadata.tokens_per_use.map(|x| x.into()));
            drop.asset_by_id.insert(&metadata.asset_id, &asset);
        }

        self.drop_by_id.insert(&drop_id, &drop);
    }
}