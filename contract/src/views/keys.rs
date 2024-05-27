use crate::*;

#[near_bindgen]
impl Keypom {
    /// Allows you to query for the amount of $NEAR tokens contained in a linkdrop corresponding to a given public key.
    ///
    /// Requirements:
    /// * Panics if the key does not exist.
    ///
    /// Arguments:
    /// * `key` Either the public counterpart of the key used to sign, expressed as a string with format "<key-type>:<base58-key-bytes>" (e.g. "ed25519:6TupyNrcHGTt5XRLmHTc2KGaiSbjhQi1KHtCXTgbcr4Y")
    /// or the token ID corresponding to the key.
    ///
    /// Returns a string representing the $yoctoNEAR amount associated with a given public key
    #[handle_result]
    pub fn get_key_balance(&self, key: String) -> Result<U128, String> {
        Ok(self.get_key_information(key)?.yoctonear)
    }

    /// Allows you to query for the `KeyInfo` corresponding to a given public key. This method is preferred over `get_key_balance` as it provides more information about the key.
    ///
    /// Requirements:
    /// * Panics if the key does not exist.
    ///
    /// Arguments:
    /// * `key` Either the public counterpart of the key used to sign, expressed as a string with format "<key-type>:<base58-key-bytes>" (e.g. "ed25519:6TupyNrcHGTt5XRLmHTc2KGaiSbjhQi1KHtCXTgbcr4Y")
    /// or the token ID corresponding to the key.
    ///
    /// Returns `KeyInfo` associated with a given public key
    #[handle_result]
    pub fn get_key_information(&self, key: String) -> Result<ExtKeyInfo, String> {
        let token_id = self.parse_key_or_token_id(key);
        let (drop_id, _) = parse_token_id(&token_id)?;

        let drop = self
            .drop_by_id
            .get(&drop_id)
            .expect("no drop found for drop ID");
        let key_info = drop
            .key_info_by_token_id
            .get(&token_id)
            .expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let InternalAssetDataForUses {
            uses: _,
            config: use_config,
            assets_metadata,
            required_asset_gas,
        } = get_asset_data_for_specific_use(&drop.asset_data_for_uses, &cur_key_use);

        // If the config's permission field is set to Claim, the base should be set accordingly. In all other cases, it should be the base for CAAC
        let base_gas_for_use =
            if let Some(perms) = use_config.as_ref().and_then(|c| c.permissions.as_ref()) {
                match perms {
                    ClaimPermissions::claim => BASE_GAS_FOR_CLAIM,
                    _ => BASE_GAS_FOR_CREATE_ACC_AND_CLAIM,
                }
            } else {
                BASE_GAS_FOR_CREATE_ACC_AND_CLAIM
            };

        // Keep track of the total gas across all assets in the current use
        let required_gas: Gas =
            Gas::from_gas(base_gas_for_use.as_gas() + required_asset_gas.as_gas());

        let mut ft_list: Vec<FTListData> = Vec::new();
        let mut nft_list: Vec<NFTListData> = Vec::new();
        let mut fc_list: Vec<FCData> = Vec::new();

        let mut yoctonear = 0;
        let mut num_nfts = 0;
        for metadata in assets_metadata {
            let internal_asset = drop
                .asset_by_id
                .get(&metadata.asset_id)
                .expect("Asset not found");

            match internal_asset {
                InternalAsset::ft(ft) => {
                    ft_list.push(FTListData {
                        amount: metadata.tokens_per_use.unwrap().0.to_string(),
                        contract_id: ft.contract_id.to_string(),
                    });
                }
                InternalAsset::nft(nft) => {
                    let last_idx = nft.token_ids.len().saturating_sub(1);
                    let idx = last_idx
                        .checked_sub(num_nfts)
                        .unwrap_or(nft.token_ids.len());

                    if let Some(token_id) = nft.token_ids.get(idx) {
                        nft_list.push(NFTListData {
                            token_id: token_id.to_string(),
                            contract_id: nft.contract_id.to_string(),
                        });
                    }

                    num_nfts += 1;
                }
                InternalAsset::fc(fc) => {
                    fc_list.push(fc.clone());
                }
                InternalAsset::near => {
                    yoctonear += metadata.tokens_per_use.unwrap().0;
                }
                InternalAsset::none => {}
            }
        }

        Ok(ExtKeyInfo {
            yoctonear: U128(yoctonear),
            ft_list,
            nft_list,
            fc_list,
            metadata: key_info.metadata,
            required_gas: required_gas.as_gas().to_string(),
            uses_remaining: key_info.remaining_uses,
            drop_id,
            token_id,
            pub_key: key_info.pub_key,
            owner_id: key_info.owner_id.unwrap_or(env::current_account_id()),
        })
    }

    /// Allows you to query for the total number of keys currently active on the Keypom contract
    ///
    ///
    /// Returns a `u64` representing the total number of keys currently active on the Keypom contract
    pub fn get_key_total_supply(&self) -> u64 {
        self.token_id_by_pk.len()
    }

    /// Allows you to paginate through all the active keys on the Keypom contract regardless of which drop they are part of.
    ///
    ///
    /// Arguments:
    /// * `from_index` where to start paginating from. If not specified, will start from 0 index.
    /// * `limit` how many keys to return. If not specified, will return 50 keys.
    ///
    /// Returns a vector of `ExtKeyInfo` objects representing the information about the keys
    #[handle_result]
    pub fn get_keys(
        &self,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Result<Vec<ExtKeyInfo>, String> {
        let start = u128::from(from_index.unwrap_or(U128(0)));

        self.token_id_by_pk
            .values()
            .skip(start as usize)
            .take(limit.unwrap_or(50) as usize)
            .map(|token_id| self.get_key_information(token_id))
            .collect()
    }

    /// Allows you to query for the information about a batch of keys all at once with 1 function.
    ///
    ///
    /// Arguments:
    /// * `key` Either the public counterpart of the key used to sign, expressed as a string with format "<key-type>:<base58-key-bytes>" (e.g. "ed25519:6TupyNrcHGTt5XRLmHTc2KGaiSbjhQi1KHtCXTgbcr4Y")
    /// or the token ID corresponding to the key.
    ///
    /// Returns a vector of optional `ExtKeyInfo` objects representing the information about the keys. If
    /// Any of the keys do not exist, the corresponding index in the vector will be `None`
    pub fn get_key_information_batch(&self, keys: Vec<String>) -> Vec<Option<ExtKeyInfo>> {
        keys.iter()
            .map(|key| self.get_key_information(key.clone()).ok())
            .collect()
    }
}
