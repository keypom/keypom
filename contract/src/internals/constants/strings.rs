// ------------------------ Access Key Method Names ------------------------ //
/// Specifies both `claim` and `create_account_and_claim` functions can be called with the access key
pub const ACCESS_KEY_BOTH_METHOD_NAMES: &str = "claim,create_account_and_claim,nft_transfer,nft_approve,nft_revoke";
/// Specifies only `claim` can be called with the access key
pub const ACCESS_KEY_CLAIM_METHOD_NAME: &str = "claim,nft_transfer,nft_approve,nft_revoke";
/// Specifies only `create_account_and_claim` can be called with the access key
pub const ACCESS_KEY_CREATE_ACCOUNT_METHOD_NAME: &str = "create_account_and_claim,nft_transfer,nft_approve,nft_revoke";


// ------------------------ NFT Standard Stuff ------------------------ //
/// This spec can be treated like a version of the standard.
pub const NFT_METADATA_SPEC: &str = "1.0.0";
/// This is the name of the NFT standard we're using
pub const NFT_STANDARD_NAME: &str = "nep171";