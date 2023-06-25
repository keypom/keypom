use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, LookupSet, UnorderedMap};
use near_sdk::json_types::U128;
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, PublicKey, Promise, PromiseOrValue, require, CryptoHash, Gas};

mod fungible_tokens;
mod internals;
mod models;
mod helpers;

use fungible_tokens::*;
use internals::*;
use helpers::*;
use models::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Keypom {
    drop_by_id: LookupMap<DropId, InternalDrop>
}

#[near_bindgen]
impl Keypom {
    #[init]
    pub fn new() -> Self {
        Self {
            drop_by_id: LookupMap::new(StorageKeys::DropById)
        }
    }

    #[payable]
    pub fn create_drop(&mut self, drop_id: String, public_keys: Vec<PublicKey>, assets_per_use: HashMap<UseNumber, Vec<ExtAsset>>) {
        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);
        
        // Instantiate the two lookup maps used in the drop
        let mut assets_metadata_by_use: LookupMap<UseNumber, Vec<AssetMetadata>> = LookupMap::new(StorageKeys::AssetIdsByUse {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });
        let mut asset_by_id: UnorderedMap<AssetId, InternalAsset> = UnorderedMap::new(StorageKeys::AssetById {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });

        // Keep track of the total cost of all assets for 1 key
        let mut per_key_cost_from_assets = 0;

        // Keep track of how much allowance each key needs
        let mut total_allowance_required_per_key = 0;

        let uses_per_key = assets_per_use.len() as UseNumber;
        // Iterate through the external assets, convert them to internal assets and add them to both lookup maps
        for (use_number, ext_assets) in assets_per_use {
            // Quick sanity check to make sure the use number is valid
            require!(use_number <= uses_per_key && use_number > 0, "Invalid use number");

            // Keep track of the metadata for all the assets across each use
            let mut assets_metadata: Vec<AssetMetadata> = Vec::new();

            // Keep track of the total gas across all assets in a given use
            let mut total_gas_for_use: Gas = Gas(0);
            
            // If there's assets, loop through and get all the asset IDs while also
            // adding them to the asset_by_id lookup map if they weren't already present
            // If there aren't any assets, the vector will be of length 1
            for ext_asset in ext_assets {
                let asset_id = asset_id_from_ext_asset(&ext_asset);
                let tokens_per_use = ext_asset.get_tokens_per_use();

                assets_metadata.push(AssetMetadata {
                    asset_id: asset_id.clone(),
                    tokens_per_use: tokens_per_use.into()
                });

                // Every asset has a cost associated. We should add that to the total cost.
                // This is for 1 key. At the end, we'll multiply by the number of keys
                let cost_for_asset = ExtAsset::get_cost_per_key(&ext_asset);
                near_sdk::log!("cost for asset {}", cost_for_asset);
                per_key_cost_from_assets += cost_for_asset;

                // Every asset has a gas cost associated. We should add that to the total gas.
                let gas_for_asset = ExtAsset::get_gas_for_asset(&ext_asset);
                near_sdk::log!("gas for asset {:?}", gas_for_asset);
                total_gas_for_use += gas_for_asset;

                // Only insert into the asset ID map if it doesn't already exist
                // If we insert, we should also add the cost to the total asset cost
                if asset_by_id.get(&asset_id).is_none() {
                    let internal_asset = ext_asset.to_internal_asset();

                    asset_by_id.insert(&asset_id, &internal_asset);
                }
            }

            // Now that all the assets have been looped for the given use, we can get the allowance required
            total_allowance_required_per_key += calculate_base_allowance(YOCTO_PER_GAS, total_gas_for_use);

            assets_metadata_by_use.insert(&use_number, &assets_metadata);
        }

        let drop = InternalDrop {
            uses_per_key,
            assets_metadata_by_use,
            asset_by_id
        };

        self.drop_by_id.insert(&drop_id, &drop);

        let final_storage = env::storage_usage();
        let storage_cost = (final_storage - initial_storage) as u128 * env::storage_byte_cost();
        let num_keys = public_keys.len();
        let total_asset_cost = per_key_cost_from_assets * num_keys as u128;
        let total_cost = total_asset_cost + storage_cost;
        near_sdk::log!("total {} storage {} asset {}", total_cost, storage_cost, total_asset_cost);

        internal_refund_excess_deposit(total_cost);
        internal_add_keys_to_account(&public_keys, ACCESS_KEY_BOTH_METHOD_NAMES, total_allowance_required_per_key);
    }

    #[payable]
    pub fn claim(&mut self, drop_id: DropId, use_number: UseNumber, receiver_id: AccountId) {
        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let assets_metadata = drop.assets_metadata_by_use.get(&use_number).expect("Use number not found");

        for metadata in assets_metadata {
            let mut asset: InternalAsset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            asset.claim_asset(&drop_id, &receiver_id, &metadata.tokens_per_use.map(|x| x.into()));
            drop.asset_by_id.insert(&metadata.asset_id, &asset);
        }

        self.drop_by_id.insert(&drop_id, &drop);
    }

    /// Allows users to attach fungible tokens to the Linkdrops. Must have storage recorded by this point. You can only attach one set of FTs or NFT at a time.
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
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
        };

        drop.asset_by_id.insert(&asset_id.to_string(), &asset);
        self.drop_by_id.insert(&drop_id, &drop);

        PromiseOrValue::Value(U128(0))
    }

    pub fn get_drop_information(&self, drop_id: DropId) -> Option<ExtDrop> {
        if let Some(drop) = self.drop_by_id.get(&drop_id) {
            return Some(ExtDrop::from_internal_drop(&drop));
        } else {
            None
        }
    }
}


// {
//     dataTypes: {
//         "go_team_token": {FT_DATA},
//         "nft_poap": {NFT_DATA}
//     },
//     uses: {
//         useOne: ["id1", "id2", "id3"]
//         useTwo: ["id1", "id1"],
//         useThree: null
//     }
// }

// {
//     dataTypes: {
//         FT_DATA: [use1, use2, use4]
//         NFT_DATA: [use4]
//     }
// }

// {
//     uses: {
//         useOne: [FT_DATA, FT_DATA, NFT_DATA],
//         useTwo: [NFT_DATA],
//         useThree: null

//     }
// }