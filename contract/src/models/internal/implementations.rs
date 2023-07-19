use crate::*;

impl InternalDrop {
    /// Convert an `InternalDrop` into an `ExtDrop`
    pub fn to_external_drop(&self) -> ExtDrop {
        let mut assets_per_use: HashMap<UseNumber, Vec<Option<ExtAsset>>> = HashMap::new();

        let mut nft_list = vec![];
        let mut ft_list = vec![];
        let mut fc_list = vec![];

        // Loop through all the values in the asset_by_id hashmap and add them to the corresponding vectors
        self.asset_by_id.values().for_each(|asset| {
            match asset {
                InternalAsset::nft(nft_asset) => nft_list.push(nft_asset),
                InternalAsset::ft(ft_asset) => ft_list.push(ft_asset),
                InternalAsset::fc(fc_asset) => fc_list.push(fc_asset),
                _ => {}
            }
        });

        // Loop through starting from 1 -> max_num_uses and add the assets to the hashmap
        for use_number in 1..=self.uses_per_key {
            let KeyBehavior {assets_metadata, config: _} = self.key_behavior_by_use.get(&use_number).expect("Use number not found");

            let mut assets: Vec<Option<ExtAsset>> = Vec::new();
            
            for metadata in assets_metadata {
                let asset = self.asset_by_id.get(&metadata.asset_id).unwrap();
                assets.push(asset.to_external_asset(&metadata));
            }
            assets_per_use.insert(use_number, assets);
        }

        ExtDrop {
            assets_per_use,
            nft_asset_data: nft_list,
            ft_asset_data: ft_list,
            fc_asset_data: fc_list,
            metadata: self.metadata.get()
        }
    }
}

impl InternalAsset {
    /// Convert an `InternalAsset` into an `ExtAsset`
    pub fn to_external_asset(self, asset_metadata: &AssetMetadata) -> Option<ExtAsset> {
        match self {
            InternalAsset::ft(ft_data) => Some(ExtAsset::FTAsset(ExtFTData {
                ft_contract_id: ft_data.contract_id.clone(),
                registration_cost: ft_data.registration_cost.into(),
                // FTs should ALWAYS have a tokens_per_use value
                ft_amount: asset_metadata.tokens_per_use.unwrap().into()
            })),
            InternalAsset::nft(nft_data) => Some(ExtAsset::NFTAsset(ExtNFTData {
                nft_contract_id: nft_data.contract_id.clone()
            })),
            InternalAsset::fc(fc_data) => Some(ExtAsset::FCAsset(FCData::new(fc_data.methods))),
            InternalAsset::near => Some(ExtAsset::NearAsset(ExtNEARData { yoctonear: asset_metadata.tokens_per_use.unwrap().into() })),
            InternalAsset::none => None
        }
    }

    /// Standard function for claiming an asset regardless of its type
    /// This will return a promise for every asset that will be resolved in a standard callback
    pub fn claim_asset(&mut self, receiver_id: &AccountId, tokens_per_use: &Option<Balance>) -> Promise {
        match self {
            InternalAsset::ft(ref mut ft_data) => {
                return ft_data.claim_ft_asset(receiver_id, &tokens_per_use.unwrap())
            },
            InternalAsset::nft(ref mut nft_data) => {
                return nft_data.claim_nft_asset(receiver_id)
            },
            InternalAsset::fc(ref mut fc_data) => {
                return fc_data.claim_fc_asset()
            },
            InternalAsset::near => {
                return Promise::new(receiver_id.clone()).transfer(tokens_per_use.unwrap());
            },
            InternalAsset::none => {
                return Promise::new(env::current_account_id());
            }
        }
    }

    /// Standard function outlining what should happen if a specific claim failed
    /// This should return the amount of $NEAR that should be refunded to the user's balance
    /// In addition, any internal state changes should be made (i.e balance_available incremented for FTs)
    pub fn on_failed_claim(&mut self, tokens_per_use: &Option<String>) -> Balance {
        match self {
            InternalAsset::ft(ref mut ft_data) => {
                let ft_to_refund = &tokens_per_use.as_ref().unwrap().parse::<u128>().unwrap();
                near_sdk::log!("Failed claim for FT asset. Refunding {} to the user's balance and incrementing balance available by {}", 0, ft_to_refund);
                ft_data.add_to_balance_avail(ft_to_refund);
                ft_data.registration_cost
            },
            InternalAsset::nft(ref mut nft_data) => {
                let token_id = &tokens_per_use.as_ref().unwrap();
                near_sdk::log!("Failed claim NFT asset with Token ID {}", token_id);
                nft_data.add_to_token_ids(token_id);
                0
            },
            InternalAsset::near => {
                let near_tokens = tokens_per_use.as_ref().unwrap().parse::<u128>().unwrap();
                near_sdk::log!("Failed claim for NEAR asset. Refunding {} to the user's balance", near_tokens);
                near_tokens
            },
            InternalAsset::fc(_) => {
                near_sdk::log!("Failed claim for FC asset. Cannot refund.");
                0
            },
            InternalAsset::none => {
                near_sdk::log!("Failed claim for null asset. SHOULD NEVER HAPPEN");
                0
            }
        }
    }

    /// Standard function to check whether an asset is empty or not
    pub fn is_empty(&self) -> bool {
        match self {
            InternalAsset::ft(ft) => ft.enough_balance(&1),
            InternalAsset::nft(nft) => nft.is_empty(),
            InternalAsset::fc(_) => true,
            InternalAsset::near => true,
            InternalAsset::none => true
        }
    }

    /// Standard function for refunding assets
    /// This does not include any ext assets such as FTs or NFTs.
    /// This simply refunds the funder for the $NEAR cost associated with 1 key use for the given asset
    pub fn get_yocto_refund_amount(&self, tokens_per_use: &Option<Balance>) -> Balance {
        match self {
            InternalAsset::ft(ft_data) => {
                return ft_data.registration_cost;
            },
            InternalAsset::nft(_) => 0,
            InternalAsset::near => {
                return tokens_per_use.unwrap();
            },
            InternalAsset::fc(fc_data) => fc_data.get_yocto_refund_amount(),
            InternalAsset::none => 0
        }
    }

    /// Standard function to query how much gas it takes for 1 claim of a given asset
    pub fn get_required_gas(&self) -> Gas {
        match self {
            InternalAsset::ft(ft_data) => ft_data.get_required_gas_for_claim(),
            InternalAsset::nft(nft_data) => nft_data.get_required_gas_for_claim(),
            InternalAsset::fc(fc_data) => fc_data.get_required_gas_for_claim(),
            InternalAsset::near => GAS_FOR_NEAR_TRANSFER,
            InternalAsset::none => GAS_FOR_NONE_ASSET
        }
    }
}