use crate::*;
use near_sdk::collections::{UnorderedMap, Vector};

/// Internal drop data that is stored in the contract
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalDrop {
    /// Account ID who funded / owns the rights to this specific drop
    pub funder_id: AccountId,

    /// What is the maximum number of uses a given key can have in the drop?
    pub max_key_uses: UseNumber,
    /// Map an asset ID to a specific asset. This is a hyper optimization so the asset data isn't repeated in the contract
    pub asset_by_id: UnorderedMap<AssetId, InternalAsset>,
    /// For every use, keep track of what assets there are (in order)
    pub asset_data_for_uses: Vector<InternalAssetDataForUses>,

    /// Set of public keys associated with this drop mapped to their specific key information.
    pub key_info_by_token_id: UnorderedMap<TokenId, InternalKeyInfo>,
    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64,

    /// Keep track of different configuration options for all the uses of a key in a given drop
    pub config: Option<DropConfig>
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalKeyInfo {
    /// Current public key that is mapped to this key info
    pub pub_key: PublicKey,

    /// How many uses this key has left. Once 0 is reached, the key is deleted
    pub remaining_uses: UseNumber,

    /// Owner of the key
    pub owner_id: AccountId,

    /// When was the last time the key was used
    pub last_claimed: u64,

    /// Metadata for the current key
    pub metadata: Option<String>,
    pub pw_by_use: Option<HashMap<UseNumber, Vec<u8>>>,

    /// List of approved account IDs that have access to transfer the token. This maps an account ID to an approval ID
    pub approved_account_ids: HashMap<AccountId, u64>,

    /// The next approval ID to give out. 
    pub next_approval_id: u64,
}

/// Outlines the asset data for a set of uses
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct InternalAssetDataForUses {
    /// How many uses does this asset data apply to?
    pub uses: UseNumber,
    /// Any configurations for this set of uses
    pub config: Option<UseConfig>,
    /// The actual assets themselves (ID + tokens)
    pub assets_metadata: Vec<AssetMetadata>
}

impl From<ExtAssetDataForUses> for InternalAssetDataForUses {
    fn from(ext_asset_data: ExtAssetDataForUses) -> Self {
        let mut assets_metadata = vec![];

        let mut fc_asset_id = 0;
        for ext_asset in ext_asset_data.assets {
            // If the external asset is of type FCData, the asset ID will be the incrementing number
            // Otherwise, it will be the asset ID specified
            let asset_id = if let Some(ExtAsset::FCAsset(_)) = ext_asset {
                fc_asset_id += 1;
                fc_asset_id.to_string()
            } else {
                ext_asset.as_ref().and_then(|a| Some(a.get_asset_id())).unwrap_or(NONE_ASSET_ID.to_string())
            };

            assets_metadata.push(AssetMetadata {
                asset_id: asset_id.clone(),
                tokens_per_use: ext_asset.as_ref().and_then(|a| Some(a.get_tokens_per_use()))
            });
        }


        Self {
            uses: ext_asset_data.uses,
            config: ext_asset_data.config,
            assets_metadata
        }
    }
}

/// Outlines the different asset types that can be used in drops. This is the internal version of `ExtAsset`
/// And represents the data that is stored inside the Keypom contract to keep track of assets
#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum InternalAsset {
    ft(InternalFTData),
    nft(InternalNFTData),
    fc(FCData),
    near,
    none
}

/// Metadata corresponding to a specific asset. This keeps track of the ID and optionally tokens being transferred per use
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct AssetMetadata {
    /// What asset is mapped to this specific use
    pub asset_id: AssetId,
    /// How many tokens should be transferred in this use?
    /// This is only relevant for FT and $NEAR assets
    pub tokens_per_use: Option<U128>,
}

/// Contract metadata structure
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractSourceMetadata {
    /// Commit hash being used for the currently deployed wasm. If the contract is not open-sourced, this could also be a numbering system for internal organization / tracking such as "1.0.0" and "2.1.0".
    pub version: String,
    /// Link to open source code such as a Github repository or a CID to somewhere on IPFS.
    pub link: String,
}

/// Fee Structures for drops and keys
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct KeypomFees {
    /// How much $NEAR users are charged for creating a drop
    pub per_drop: u128,
    /// How much $NEAR users are charged for adding a key
    pub per_key: u128,
}

/// Data returned from the `before_claim_logic` function
pub struct BeforeClaimData {
    /// What is the token ID for the key being claimed
    pub token_id: TokenId,
    /// How much gas the assets in the given use require
    pub required_asset_gas: Gas,
    /// For CAAC, there needs to be a root for all accounts. By default, this is the contract's global root account (i.e `near` or `testnet`) but if otherwise specified in the use or drop config, it will be that.
    pub root_account_id: AccountId,
    /// When calling `create_account` on the root account, which keypom args should be attached to the payload.
    pub account_creation_keypom_args: Option<KeypomInjectedArgs>,
}

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKeys {
    AssetById { drop_id_hash: CryptoHash },
    KeyInfoByPk { drop_id_hash: CryptoHash },
    AssetDataForUses { drop_id_hash: CryptoHash },
    TokensPerOwnerInner { account_id_hash: CryptoHash },
    DropIdsByFunderInner { account_id_hash: CryptoHash },
    DropIdsByFunder,
    FeesPerUser,
    ContractMetadata,
    TokensPerOwner,
    DropById,
    TokenIdByPk,
    UserBalances
}