use crate::*;

#[near_bindgen]
impl Keypom {
    /// Returns the drop information associated with given drop ID.
    pub fn get_drop_information(&self, drop_id: DropId) -> Option<ExtDrop> {
        if let Some(drop) = self.drop_by_id.get(&drop_id) {
            return Some(ExtDrop::from_internal_drop(&drop));
        } else {
            None
        }
    }

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
        let KeyBehavior {assets_metadata, config: _} = drop.key_behavior_by_use.get(&cur_key_use).expect("Use number not found");

        let mut required_gas = BASE_GAS_FOR_CLAIM;

        let mut actual_ft_list: Vec<ExtFTData> = Vec::new();
        for metadata in assets_metadata {
            let internal_asset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            let ext_asset = ExtAsset::from_internal_asset(&internal_asset, &metadata);
            required_gas += ext_asset.get_required_gas();
            
            match ext_asset {
                ExtAsset::FTAsset(ft) => {
                    actual_ft_list.push(ft);
                }
            }
        }

        ExtKeyInfo {
            yoctonear: 0.to_string(),
            ft_list: if actual_ft_list.len() > 0 { Some(actual_ft_list) } else { None },
            required_gas: u64::from(required_gas).to_string(),
        }
    }

    /// Returns the total supply of active keys for a given drop
    pub fn get_key_supply_for_drop(&self, drop_id: DropId) -> u64 {
        // Get the drop object and return the length
        self.drop_by_id
            .get(&drop_id)
            .expect("no drop found")
            .key_info_by_pk
            .len()
    }

    /// Paginate through keys in a specific drop
    pub fn get_keys_for_drop(
        &self,
        drop_id: DropId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<ExtKeyInfo> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        let drop = self.drop_by_id
            .get(&drop_id)
            .expect("No drop for given ID");
        
        return drop
            .key_info_by_pk
            .keys()
            //skip to the index we specified in the start variable
            .skip(start as usize)
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize)
            //we'll map the public key which are strings into Drops
            .map(|pk| self.get_key_information(pk))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }
}