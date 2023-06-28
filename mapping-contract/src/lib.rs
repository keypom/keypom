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
mod views;

use fungible_tokens::*;
use internals::*;
use helpers::*;
use models::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Keypom {
    /// Map a drop ID to its internal drop data
    drop_by_id: LookupMap<DropId, InternalDrop>,
    /// Map of each key to its respective drop ID. This is much more efficient than repeating the
    /// Drop data for every single key.
    drop_id_for_pk: UnorderedMap<PublicKey, DropId>,
}

#[near_bindgen]
impl Keypom {
    #[init]
    pub fn new() -> Self {
        Self {
            drop_by_id: LookupMap::new(StorageKeys::DropById),
            drop_id_for_pk: UnorderedMap::new(StorageKeys::DropIdByPk),
        }
    }

    #[payable]
    pub fn create_drop(&mut self, drop_id: String, public_keys: Vec<PublicKey>, assets_per_use: HashMap<UseNumber, Vec<ExtAsset>>) {
        // Before anything, measure storage usage so we can net the cost and charge the funder
        let initial_storage = env::storage_usage();
        near_sdk::log!("initial bytes {}", initial_storage);
        
        // Instantiate the drop data structures
        let mut assets_metadata_by_use: LookupMap<UseNumber, Vec<AssetMetadata>> = LookupMap::new(StorageKeys::AssetIdsByUse {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });
        let mut asset_by_id: UnorderedMap<AssetId, InternalAsset> = UnorderedMap::new(StorageKeys::AssetById {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });
        let mut key_info_by_pk: UnorderedMap<PublicKey, InternalKeyInfo> = UnorderedMap::new(StorageKeys::KeyInfoByPk {
            drop_id_hash: hash_drop_id(&drop_id.to_string()),
        });

        // Keep track of the total cost of all assets for 1 key
        let mut per_key_cost_from_assets = 0;

        // Keep track of how much allowance each key needs
        let mut total_allowance_required_per_key = 0;
        // Parse the external assets and store them in the contract
        let uses_per_key = assets_per_use.len() as UseNumber;
        parse_ext_assets_per_use(
            uses_per_key, 
            assets_per_use,
            &mut assets_metadata_by_use, 
            &mut asset_by_id, 
            &mut total_allowance_required_per_key, 
            &mut per_key_cost_from_assets
        );

        // Keep track of all the key IDs 
        let mut next_key_id = 0;

        // Add the keys to the contract
        self.internal_add_keys_to_account(
            &mut next_key_id,
            &mut key_info_by_pk,
            &drop_id,
            uses_per_key,
            &public_keys, 
            ACCESS_KEY_BOTH_METHOD_NAMES, 
            total_allowance_required_per_key
        );

        // Write the drop data to storage
        let drop = InternalDrop {
            uses_per_key,
            assets_metadata_by_use,
            asset_by_id,
            key_info_by_pk,
            next_key_id
        };
        self.drop_by_id.insert(&drop_id, &drop);

        // Measure final storage costs
        let final_storage = env::storage_usage();
        let storage_cost = (final_storage - initial_storage) as u128 * env::storage_byte_cost();
        let num_keys = public_keys.len() as u128;
        let total_asset_cost = per_key_cost_from_assets * num_keys;
        let total_allowance_cost = total_allowance_required_per_key * num_keys;
        let total_cost = total_asset_cost + storage_cost + total_allowance_cost;
        near_sdk::log!("total {} storage {} asset {} allowance {}", total_cost, storage_cost, total_asset_cost, total_allowance_cost);
        internal_refund_excess_deposit(total_cost);
    }

    #[payable]
    pub fn claim(&mut self, receiver_id: AccountId) {
        let signer_pk = env::signer_account_pk();
        let drop_id = self.drop_id_for_pk.get(&signer_pk).expect("Drop not found");

        let mut drop: InternalDrop = self.drop_by_id.get(&drop_id).expect("Drop not found");
        let key_info = drop.key_info_by_pk.get(&signer_pk).expect("Key not found");
        let cur_key_use = get_key_cur_use(&drop, &key_info);
        let assets_metadata = drop.assets_metadata_by_use.get(&cur_key_use).expect("Use number not found");

        for metadata in assets_metadata {
            let gas_used = env::used_gas();
            let mut asset: InternalAsset = drop.asset_by_id.get(&metadata.asset_id).expect("Asset not found");
            asset.claim_asset(&drop_id, &receiver_id, &metadata.tokens_per_use.map(|x| x.into()));
            drop.asset_by_id.insert(&metadata.asset_id, &asset);
            near_sdk::log!("gas used 3{:?}", env::used_gas() - gas_used);
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