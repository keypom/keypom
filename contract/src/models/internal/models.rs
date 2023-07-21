use near_sdk::collections::{UnorderedMap, LazyOption};

use crate::*;

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
    pub key_behavior_by_use: LookupMap<UseNumber, InternalKeyBehavior>,

    /// Information about the NFT keys and how they're rendered / payout options etc.
    pub nft_config: Option<NFTKeyBehaviour>,

    /// Set of public keys associated with this drop mapped to their specific key information.
    pub key_info_by_token_id: UnorderedMap<TokenId, InternalKeyInfo>,
    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64
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

    // List of approved account IDs that have access to transfer the token. This maps an account ID to an approval ID
    pub approved_account_ids: HashMap<AccountId, u64>,

    // The next approval ID to give out. 
    pub next_approval_id: u64,
}

/// Every use number has corresponding behavior data which includes information about all the assets in that use
#[derive(BorshDeserialize, BorshSerialize)]
pub struct InternalKeyBehavior {
    /// Configurations for this specific use
    pub config: ExtConfig, // TODO
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
#[derive(BorshDeserialize, BorshSerialize)]
pub struct AssetMetadata {
    /// What asset is mapped to this specific use
    pub asset_id: AssetId,
    /// How many tokens should be transferred in this use?
    /// This is only relevant for FT and $NEAR assets
    pub tokens_per_use: Option<U128>,
}

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKeys {
    AssetIdsByUse { drop_id_hash: CryptoHash },
    AssetById { drop_id_hash: CryptoHash },
    KeyInfoByPk { drop_id_hash: CryptoHash },
    DropMetadata { drop_id_hash: CryptoHash },
    TokensPerOwnerInner { account_id_hash: CryptoHash },
    TokensPerOwner,
    DropById,
    TokenIdByPk,
    UserBalances
}