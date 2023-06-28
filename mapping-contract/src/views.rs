use crate::*;

#[near_bindgen]
impl Keypom {
    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_information(&self, key: PublicKey) -> ExtKeyInfo {
        let drop_id = self
            .drop_id_for_pk
            .get(&key)
            .expect("no drop ID found for key");
        let drop = self
            .drop_by_id
            .get(&drop_id)
            .expect("no drop found for drop ID");
        let key_info = drop.key_info_by_pk.get(&key).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let assets_metadata = drop.assets_metadata_by_use.get(&cur_key_use).expect("Use number not found");

        let mut required_gas = BASE_GAS_FOR_CLAIM;

        let mut ft_list: Option<Vec<ExtFTData>> = None;
        for metadata in assets_metadata {
            let internal_asset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            let ext_asset = ExtAsset::from_internal_asset(&internal_asset, &metadata);
            required_gas += ext_asset.get_gas_for_asset();
            
            match ext_asset {
                ExtAsset::FTAsset(ft) => {
                    ft_list.as_mut().and_then(|list| Some(list.push(ft)));
                }
            }
        }

        ExtKeyInfo {
            yoctonear: 0.to_string(),
            ft_list,
            required_gas: u64::from(required_gas).to_string(),
        }
    }
}