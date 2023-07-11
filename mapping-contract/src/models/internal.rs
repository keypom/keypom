use near_sdk::collections::{UnorderedMap, LazyOption};

use crate::*;

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKeys {
    AssetIdsByUse { drop_id_hash: CryptoHash },
    AssetById { drop_id_hash: CryptoHash },
    KeyInfoByPk { drop_id_hash: CryptoHash },
    DropMetadata { drop_id_hash: CryptoHash },
    DropById,
    DropIdByPk,
    UserBalances
}

/// Internal drop data that is stored in the contract
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalDrop {
    /// Account ID who funded / owns the rights to this specific drop
    pub funder_id: AccountId,
    /// Metadata for the drop in the form of stringified JSON. The format is completely up to the
    /// user and there are no standards for format.
    pub metadata: LazyOption<DropMetadata>,
    /// How many uses there are per key in the drop. This should be equal to the length of keys in assets_metadata_by_use
    pub uses_per_key: UseNumber,
    
    /// Map an asset ID to a specific asset. This is a hyper optimization so the asset data isn't repeated in the contract
    pub asset_by_id: UnorderedMap<AssetId, InternalAsset>,
    /// For every use number, keep track of what assets there are.
    pub key_behavior_by_use: LookupMap<UseNumber, KeyBehavior>,
    
    /// Set of public keys associated with this drop mapped to their specific key information.
    pub key_info_by_pk: UnorderedMap<PublicKey, InternalKeyInfo>,
    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64
}

impl InternalDrop {
    /// Convert an `InternalDrop` into an `ExtDrop`
    pub fn to_external_drop(&self) -> ExtDrop {
        let mut assets_per_use: HashMap<UseNumber, Vec<Option<ExtAsset>>> = HashMap::new();
        let internal_assets_data: Vec<InternalAsset> = self.asset_by_id.values_as_vector().to_vec();
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
            internal_assets_data,
            metadata: self.metadata.get()
        }
    }
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalKeyInfo {
    /// How many uses this key has left. Once 0 is reached, the key is deleted
    pub remaining_uses: UseNumber,

    /// Nonce for the current key.
    pub key_id: u64,
}

/// Outlines the different asset types that can be used in drops. This is the internal version of `ExtAsset`
/// And represents the data that is stored inside the Keypom contract to keep track of assets
#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize, Debug, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum InternalAsset {
    ft(InternalFTData),
    nft(InternalNFTData),
    near,
    none
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
                0
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
            InternalAsset::near => true,
            InternalAsset::none => true
        }
    }

    /// Standard function for refunding assets
    /// This does not include any ext assets such as FTs or NFTs.
    /// This simply refunds the funder for the $NEAR cost associated with 1 key use for the given asset
    pub fn refund_amount(&self, tokens_per_use: &Option<Balance>) -> Balance {
        match self {
            InternalAsset::ft(ft_data) => {
                return ft_data.registration_cost;
            },
            InternalAsset::nft(_) => 0,
            InternalAsset::near => {
                return tokens_per_use.unwrap();
            },
            InternalAsset::none => 0
        }
    }

    /// Standard function to query how much gas it takes for 1 claim of a given asset
    pub fn get_required_gas(&self) -> Gas {
        match self {
            InternalAsset::ft(ft_data) => ft_data.get_required_gas_for_claim(),
            InternalAsset::nft(nft_data) => nft_data.get_required_gas_for_claim(),
            InternalAsset::near => GAS_FOR_NEAR_TRANSFER,
            InternalAsset::none => GAS_FOR_NONE_ASSET
        }
    }
}

/// Every use number has corresponding behavior data which includes information about all the assets in that use
#[derive(BorshDeserialize, BorshSerialize)]
pub struct KeyBehavior {
    /// Configurations for this specific use
    pub config: Option<bool>, // TODO
    /// Metadata for each asset in this use
    pub assets_metadata: Vec<AssetMetadata>
}

/// Metadata corresponding to a specific asset. This keeps track of the ID and optionally tokens being transferred per use
#[derive(BorshDeserialize, BorshSerialize)]
pub struct AssetMetadata {
    /// What asset is mapped to this specific use
    pub asset_id: AssetId,
    /// How many tokens should be transferred in this use?
    /// This is only relevant for FT and $NEAR assets
    pub tokens_per_use: Option<U128>,
}