use crate::*;

#[near_bindgen]
impl Keypom {
    /// Allows you to query for the amount of $NEAR tokens contained in a linkdrop corresponding to a given public key.
    ///
    /// Requirements:
    /// * Panics if the key does not exist.
    ///
    /// Arguments:
    /// * `key` the public counterpart of the key used to sign, expressed as a string with format "<key-type>:<base58-key-bytes>" (e.g. "ed25519:6TupyNrcHGTt5XRLmHTc2KGaiSbjhQi1KHtCXTgbcr4Y")
    ///
    /// Returns a string representing the $yoctoNEAR amount associated with a given public key
    #[handle_result]
    pub fn get_key_balance(&self, key: ExtKeyOrTokenId) -> Result<U128, String> {
        Ok(self.get_key_information(key)?.yoctonear)
    }

    /// Allows you to query for the `KeyInfo` corresponding to a given public key. This method is preferred over `get_key_balance` as it provides more information about the key.
    ///
    /// Requirements:
    /// * Panics if the key does not exist.
    ///
    /// Arguments:
    /// * `key` the public counterpart of the key used to sign, expressed as a string with format "<key-type>:<base58-key-bytes>" (e.g. "ed25519:6TupyNrcHGTt5XRLmHTc2KGaiSbjhQi1KHtCXTgbcr4Y")
    ///
    /// Returns `KeyInfo` associated with a given public key
    #[handle_result]
    pub fn get_key_information(&self, key: ExtKeyOrTokenId) -> Result<ExtKeyInfo, String> {
        let token_id = self.parse_key_or_token_id(key);
        let (drop_id, _) = parse_token_id(&token_id);

        let drop = self
            .drop_by_id
            .get(&drop_id)
            .expect("no drop found for drop ID");
        let key_info = drop.key_info_by_token_id.get(&token_id).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let InternalKeyBehaviorForUse { config: use_config, assets_metadata } = get_internal_key_behavior_for_use(&drop.key_use_behaviors, &cur_key_use);

        let usage_config = use_config.as_ref().and_then(|c| c.get_usage_config()).or(drop.drop_config.as_ref().and_then(|c| c.get_usage_config()));
        // If the config usage's permission field is set to Claim, the base should be set accordingly. In all other cases, it should be the base for CAAC
        let base_gas_for_use = if let Some(usage) = usage_config {
            match usage.permissions {
                Some(ClaimPermissions::claim) => {
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

        Ok(ExtKeyInfo {
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
        })
    }

    /// Query for the total supply of keys on the contract
    pub fn get_key_total_supply(&self) -> u64 {
        self.token_id_by_pk.len()
    }

    /// Paginate through all active keys on the contract and return a vector of key info.
    #[handle_result]
    pub fn get_keys(&self, from_index: Option<U128>, limit: Option<u64>) -> Result<Vec<ExtKeyInfo>, String> {
        let start = u128::from(from_index.unwrap_or(U128(0)));

        self.token_id_by_pk
            .values()
            .skip(start as usize)
            .take(limit.unwrap_or(50) as usize)
            .map(|token_id| self.get_key_information(ExtKeyOrTokenId::TokenId(token_id)))
            .collect()
    }

    /// Get the key information for a list of keys. If any key doesn't exist, it will be None in the vector
    pub fn get_key_information_batch(&self, keys: Vec<ExtKeyOrTokenId>) -> Vec<Option<ExtKeyInfo>> {
        keys.iter()
            .map(|key| self.get_key_information(key.clone()).ok())
            .collect()
    }
}