use crate::*;

/// Outlines the different asset types that can be used in drops. This is the external version of `InternalAsset`
/// And represents the data that is passed into and out of the Keypom contract
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum ExtAsset {
    FTAsset(ExtFTData)
}

pub(crate) fn ext_asset_to_internal(ext_asset: Option<&ExtAsset>) -> InternalAsset {
    if let Some(asset) = ext_asset {
        return asset.to_internal_asset();
    }

    return InternalAsset::none;
}

impl ExtAsset {
    /// Convert an `InternalAsset` into an `ExtAsset`
    pub fn from_internal_asset(internal_asset: &InternalAsset, asset_metadata: &AssetMetadata) -> Option<Self> {
        match internal_asset {
            InternalAsset::ft(ft_data) => Some(ExtAsset::FTAsset(ExtFTData {
                contract_id: ft_data.contract_id.clone(),
                registration_cost: ft_data.registration_cost.into(),
                // FTs should ALWAYS have a tokens_per_use value
                amount: asset_metadata.tokens_per_use.unwrap().into()
            })),
            InternalAsset::none => None
        }
    }

    /// Convert an `ExtAsset` into an `InternalAsset`
    pub fn to_internal_asset(&self) -> InternalAsset {
        match self {
            ExtAsset::FTAsset(ft_data) => InternalAsset::ft(InternalFTData::new(
                ft_data.contract_id.clone(),
                ft_data.registration_cost.into(),
            ))
        }
    }

    /// Standard function to check how many tokens a given asset transfers per use
    pub fn get_tokens_per_use(&self) -> U128 {
        match self {
            ExtAsset::FTAsset(ft_data) => ft_data.amount.into()
        }
    }

    /// Standard function to check how much $NEAR (in yocto) it costs for 1 use of a given asset
    pub fn get_cost_per_key(&self) -> Balance {
        match self {
            ExtAsset::FTAsset(ft_data) => ft_data.registration_cost.into()
        }
    }

    /// Standard function to query how much gas it takes for 1 claim of a given asset
    pub fn get_required_gas(&self) -> Gas {
        match self {
            ExtAsset::FTAsset(_) => GAS_FOR_CLAIM_LOGIC + MIN_GAS_FOR_FT_TRANSFER + MIN_GAS_FOR_STORAGE_DEPOSIT + MIN_GAS_FOR_RESOLVE_BATCH
        }
    }
}

/// Data going into or out of the Keypom contract representing the presence of fungible tokens as an asset for a drop
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtFTData {
    /// The contract that the fungible tokens originate from
    pub contract_id: AccountId,
    /// How much $NEAR (in yocto) it costs to register a new user on the fungible token contract
    pub registration_cost: U128,
    /// How many fungible tokens (in their smallest indivisible unit) should be transferred when the drop is claimed
    pub amount: U128
}

/// Drop data being returned from view calls from Keypom
#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtDrop {
    assets_per_use: HashMap<UseNumber, Vec<Option<ExtAsset>>>,
    internal_assets_data: Vec<InternalAsset>,
}

impl ExtDrop {
    /// Convert an `InternalDrop` into an `ExtDrop`
    pub fn from_internal_drop(internal_drop: &InternalDrop) -> Self {
        let mut assets_per_use: HashMap<UseNumber, Vec<Option<ExtAsset>>> = HashMap::new();
        let internal_assets_data: Vec<InternalAsset> = internal_drop.asset_by_id.values().collect();
        
        // Loop through starting from 1 -> max_num_uses and add the assets to the hashmap
        for use_number in 1..=internal_drop.uses_per_key {
            let KeyBehavior {assets_metadata, config: _} = internal_drop.key_behavior_by_use.get(&use_number).expect("Use number not found");

            let mut assets: Vec<Option<ExtAsset>> = Vec::new();
            
            for metadata in assets_metadata {
                let asset = internal_drop.asset_by_id.get(&metadata.asset_id).unwrap();
                assets.push(ExtAsset::from_internal_asset(&asset, &metadata));
            }
            assets_per_use.insert(use_number, assets);
        }

        ExtDrop {
            assets_per_use,
            internal_assets_data
        }
    }
}

/// Information about a specific public key.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ExtKeyInfo {
    /// How much Gas should be attached when the key is used to call `claim` or `create_account_and_claim`.
   /// It is up to the smart contract developer to calculate the required gas (which can be done either automatically on the contract or on the client-side).
   pub required_gas: String,

   /// yoctoNEAR$ amount that will be sent to the account that claims the linkdrop (either new or existing)
   /// when the key is successfully used.
   pub yoctonear: String,

   /// If using the FT standard extension, a set of FTData can be linked to the public key
   /// indicating that all those assets will be sent to the account that claims the linkdrop (either new or
   /// existing) when the key is successfully used.
   pub ft_list: Option<Vec<ExtFTData>>,

   /* CUSTOM */
   pub uses_remaining: UseNumber
}