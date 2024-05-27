// ------------------------ Access Key Method Names ------------------------ //
pub const ACCESS_KEY_METHOD_NAMES: &str =
    "claim,create_account_and_claim,nft_transfer,nft_approve,nft_revoke";

// ------------------------ NFT Standard Stuff ------------------------ //
/// This spec can be treated like a version of the standard.
pub const NFT_METADATA_SPEC: &str = "1.0.0";
/// This is the name of the NFT standard we're using
pub const NFT_STANDARD_NAME: &str = "nep171";

// ------------------------ Keypom Standard ------------------------ //
/// What version is the keypom standard on? This is for keypom event logging
pub const KEYPOM_STANDARD_VERSION: &str = "1.0.0";
/// What is the Keypom standard name? This is for keypom event logging
pub const KEYPOM_STANDARD_NAME: &str = "keypom";

// ------------------------ Asset IDs ------------------------ //
/// Asset ID for the fc assets (this should never be used though and is just a fallback)
pub const FALLBACK_FC_ASSET_ID: &str = "fc";
/// Asset ID for the near asset
pub const NEAR_ASSET_ID: &str = "near";
/// Asset ID for the none / null asset
pub const NONE_ASSET_ID: &str = "none-asset";
/// Prefix for all FC asset IDs
pub const FC_ASSET_PREFIX: &str = "fc&";
