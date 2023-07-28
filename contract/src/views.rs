use crate::*;

#[near_bindgen]
impl Keypom {
    /// Returns the drop information associated with given drop ID.
    pub fn get_drop_information(&self, drop_id: DropId) -> Option<ExtDrop> {
        if let Some(drop) = self.drop_by_id.get(&drop_id) {
            return Some(drop.to_external_drop());
        } else {
            near_sdk::log!("Drop {} not found!", drop_id);
            None
        }
    }

    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_information(&self, key: Option<PublicKey>, token_id: Option<String>) -> ExtKeyInfo {
        let token_id = token_id.unwrap_or_else(|| self
            .token_id_by_pk
            .get(&key.expect("Must provide a key if no token ID is provided"))
            .expect("no token ID found for key"));
        let (drop_id, _) = parse_token_id(&token_id);

        let drop = self
            .drop_by_id
            .get(&drop_id)
            .expect("no drop found for drop ID");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let InternalKeyBehaviorForUse { config: use_config, assets_metadata } = get_internal_key_behavior_for_use(&drop.key_use_behaviors, &cur_key_use);

        // If the config usage's permission field is set to Claim, the base should be set accordingly. In all other cases, it should be the base for CAAC
        let base_gas_for_use = if let Some(perm) = use_config.as_ref().and_then(|c| c.usage.as_ref()).and_then(|u| u.permissions.as_ref()).or_else(|| drop.drop_config.as_ref().and_then(|c| c.usage.as_ref()).and_then(|u| u.permissions.as_ref())) {
            match perm {
                ClaimPermissions::claim => {
                    BASE_GAS_FOR_CLAIM
                }
                _ => BASE_GAS_FOR_CREATE_ACC_AND_CLAIM
            }
        } else {
            BASE_GAS_FOR_CREATE_ACC_AND_CLAIM
        };

        // Keep track of the total gas across all assets in the current use
        let mut required_gas: Gas = base_gas_for_use;

        let mut ft_list: Vec<FTListData> = Vec::new();
        let mut nft_list: Vec<NFTListData> = Vec::new();
        let mut fc_list: Vec<FCData> = Vec::new();

        let mut yoctonear = 0;
        let mut num_nfts = 0;
        for metadata in assets_metadata {
            let internal_asset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            required_gas += internal_asset.get_required_gas();
            
            match internal_asset {
                InternalAsset::ft(ft) => {
                    ft_list.push(FTListData { 
                        amount: metadata.tokens_per_use.unwrap().0.to_string(),
                        contract_id: ft.contract_id.to_string()
                    });
                },
                InternalAsset::nft(nft) => {
                    let last_idx = nft.token_ids.len().checked_sub(1).unwrap_or(0);
                    let idx = last_idx.checked_sub(num_nfts).unwrap_or(nft.token_ids.len());

                    if let Some(token_id) = nft.token_ids.get(idx) {
                        nft_list.push(NFTListData { 
                            token_id: token_id.to_string(), 
                            contract_id: nft.contract_id.to_string(), 
                        });
                    }
                    
                    num_nfts += 1;
                },
                InternalAsset::fc(fc) => {
                    fc_list.push(fc);
                },
                InternalAsset::near => {
                    yoctonear += metadata.tokens_per_use.unwrap().0;
                },
                InternalAsset::none => {}
            }
        }

        ExtKeyInfo {
            yoctonear: U128(yoctonear),
            ft_list,
            nft_list,
            fc_list,
            required_gas: u64::from(required_gas).to_string(),
            uses_remaining: key_info.remaining_uses,
            drop_id,
            token_id,
            pub_key: key_info.pub_key,
            owner_id: key_info.owner_id,
        }
    }

    /// Returns the total supply of active keys for a given drop
    pub fn get_key_supply_for_drop(&self, drop_id: DropId) -> u64 {
        // Get the drop object and return the length
        self.drop_by_id
            .get(&drop_id)
            .expect("no drop found")
            .key_info_by_token_id
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
            .key_info_by_token_id
            .keys()
            //skip to the index we specified in the start variable
            .skip(start as usize)
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize)
            //we'll map the public key which are strings into Drops
            .map(|token_id| self.get_key_information(None, Some(token_id)))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }
}