use crate::*;
use near_sdk::collections::{UnorderedMap, LazyOption};

/// When creating a drop, assets can either be specified on a per use basis or for all uses
#[derive(BorshDeserialize, BorshSerialize)]
pub enum InternalKeyUseBehaviors {
    PerUse(Vec<InternalKeyBehaviorForUse>),
    AllUses(InternalAllUseBehaviors)
}

/// If the user wishes to specify a set of assets that is repeated across many uses, they can use
/// This struct rather than pasting duplicate data when calling `create_drop`
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalAllUseBehaviors {
    /// Which assets should be present for each use
    pub assets_metadata: Vec<AssetMetadata>,
    /// How many uses are there for this drop?
    pub num_uses: UseNumber,
}

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
    pub key_use_behaviors: InternalKeyUseBehaviors,

    /// Set of public keys associated with this drop mapped to their specific key information.
    pub key_info_by_token_id: UnorderedMap<TokenId, InternalKeyInfo>,
    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64,

    /// Metadata for the drop in the form of stringified JSON. The format is completely up to the
    /// user and there are no standards for format.
    pub metadata: LazyOption<DropMetadata>,
    /// Information about the NFT keys and how they're rendered / payout options etc.
    pub nft_keys_config: Option<NFTKeyConfigurations>,
    /// Keep track of different configuration options for all the uses of a key in a given drop
    pub drop_config: Option<DropConfig>
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalKeyInfo {
    /// Current public key that is mapped to this key info
    pub pub_key: PublicKey,

    /// How many uses this key has left. Once 0 is reached, the key is deleted
    pub remaining_uses: UseNumber,

    // Owner of the Key
    pub owner_id: AccountId,

    pub metadata: Option<String>,
    pub pw_by_use: Option<HashMap<UseNumber, Vec<u8>>>,

    // List of approved account IDs that have access to transfer the token. This maps an account ID to an approval ID
    pub approved_account_ids: HashMap<AccountId, u64>,

    // The next approval ID to give out. 
    pub next_approval_id: u64,
}

/// Every use number has corresponding behavior data which includes information about all the assets in that use
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct InternalKeyBehaviorForUse {
    /// Configurations for this specific use
    pub config: Option<ConfigForGivenUse>,
    /// Metadata for each asset in this use
    pub assets_metadata: Vec<AssetMetadata>
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

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKeys {
    AssetById { drop_id_hash: CryptoHash },
    KeyInfoByPk { drop_id_hash: CryptoHash },
    DropMetadata { drop_id_hash: CryptoHash },
    TokensPerOwnerInner { account_id_hash: CryptoHash },
    TokensPerOwner,
    DropById,
    TokenIdByPk,
    UserBalances
}