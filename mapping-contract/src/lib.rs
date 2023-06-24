
use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, LookupSet, UnorderedMap};
use near_sdk::json_types::U128;
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, PublicKey, Promise, PromiseOrValue, require, CryptoHash};

mod fungible_tokens;
mod models;
mod helpers;

use fungible_tokens::*;
use helpers::*;
use models::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Mapping {
    drop_by_id: LookupMap<DropId, InternalDrop>
}

#[near_bindgen]
impl Mapping {
    #[init]
    pub fn new() -> Self {
        Self {
            drop_by_id: LookupMap::new(StorageKeys::DropById)
        }
    }

    #[payable]
    pub fn create_drop(&mut self, drop_id: String, public_keys: Vec<PublicKey>, assets_per_use: HashMap<UseNumber, Vec<ExtAsset>>) {
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);
        
        // Instantiate the two lookup maps used in the drop
        let mut asset_ids_by_use: LookupMap<UseNumber, Vec<AssetId>> = LookupMap::new(StorageKeys::AssetIdsByUse {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });
        let mut asset_by_id: UnorderedMap<AssetId, InternalAsset> = UnorderedMap::new(StorageKeys::AssetById {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });

        let mut per_key_cost_from_assets = 0;

        let uses_per_key = assets_per_use.len() as UseNumber;
        // Iterate through the external assets, convert them to internal assets and add them to both lookup maps
        for (use_number, ext_assets) in assets_per_use {
            // Quick sanity check to make sure the use number is valid
            require!(use_number <= uses_per_key && use_number > 0, "Invalid use number");

            let mut asset_ids: Vec<AssetId> = Vec::new();
            
            // If there's assets, loop through and get all the asset IDs while also
            // adding them to the asset_by_id lookup map if they weren't already present
            // If there aren't any assets, the vector will be of length 1
            for ext_asset in ext_assets {
                let asset_id = asset_id_from_ext_asset(&ext_asset);
                asset_ids.push(asset_id.clone());

                // Every asset has a cost associated. We should add that to the total cost.
                // This is for 1 key. At the end, we'll multiply by the number of keys
                let cost_for_asset = ExtAsset::get_cost_per_key(&ext_asset);
                near_sdk::log!("cost for asset {}", cost_for_asset);
                per_key_cost_from_assets += cost_for_asset;

                // Only insert into the asset ID map if it doesn't already exist
                // If we insert, we should also add the cost to the total asset cost
                if asset_by_id.get(&asset_id).is_none() {
                    let internal_asset = InternalAsset::from_ext_asset(&ext_asset);
                    asset_by_id.insert(&asset_id, &internal_asset);
                }
            }

            asset_ids_by_use.insert(&use_number, &asset_ids);
        }

        let drop = InternalDrop {
            uses_per_key,
            asset_ids_by_use,
            asset_by_id
        };

        self.drop_by_id.insert(&drop_id, &drop);

        let final_storage = env::storage_usage();
        let storage_cost = (final_storage - initial_storage) as u128 * env::storage_byte_cost();
        let num_keys = public_keys.len();
        let total_asset_cost = per_key_cost_from_assets * num_keys as u128;
        let total_cost = total_asset_cost + storage_cost;
        near_sdk::log!("total {} storage {} asset {}", total_cost, storage_cost, total_asset_cost);
    }

    // #[payable]
    // pub fn claim_ft_data(&mut self, data_id: u64, amount: U128, receiver_id: AccountId) {
    //     let mut ft_data: InternalFTData = self.custom_struct.get(&data_id).expect("No FT data found");
    //     ft_data.ft_claim(U128(data_id.into()), receiver_id, amount.0);
    //     self.custom_struct.insert(&data_id, &ft_data);
    // }

    // /// Allows users to attach fungible tokens to the Linkdrops. Must have storage recorded by this point. You can only attach one set of FTs or NFT at a time.
    // pub fn ft_on_transfer(
    //     &mut self,
    //     sender_id: AccountId,
    //     amount: U128,
    //     msg: U128,
    // ) -> PromiseOrValue<U128> {
    //     let contract_id = env::predecessor_account_id();
    //     let data_id = msg.0 as u64;

    //     let mut ft_data: InternalFTData = self.custom_struct.get(&data_id).expect("No FT data found");
    //     require!(ft_data.contract_id == contract_id, "Incorrect FT contract ID");
    //     ft_data.increment_avail_balance(amount.0);
    //     self.custom_struct.insert(&data_id, &ft_data);

    //     PromiseOrValue::Value(U128(0))
    // }

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