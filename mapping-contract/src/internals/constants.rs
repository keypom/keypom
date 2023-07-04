use crate::*;

/// Specifies both `claim` and `create_account_and_claim` functions can be called with the access key
pub const ACCESS_KEY_BOTH_METHOD_NAMES: &str = "claim,create_account_and_claim,nft_transfer,nft_approve,nft_revoke,nft_revoke_all";

/// Specifies only `claim` can be called with the access key
pub const ACCESS_KEY_CLAIM_METHOD_NAME: &str = "claim,nft_transfer,nft_approve,nft_revoke,nft_revoke_all";

/// Specifies only `create_account_and_claim` can be called with the access key
pub const ACCESS_KEY_CREATE_ACCOUNT_METHOD_NAME: &str = "create_account_and_claim,nft_transfer,nft_approve,nft_revoke,nft_revoke_all";

/// How much yoctoNEAR is in 1 unit of Gas
pub const YOCTO_PER_GAS: Balance = 100_000_000;

/// For every cross contract call, there is a base 5 TGas cost on top of what is sent to the contract
pub const MIN_BASE_GAS_FOR_ONE_CCC: Gas = Gas(5_000_000_000_000); // 5 TGas

/// Maximum amount of Gas that can be attached to the transaction
pub const MAX_GAS_ATTACHABLE: Gas = Gas(300_000_000_000_000); // 300 TGas

/// The minimum amount of Gas it costs to spin up the receipt
/// And execute claim code (outside of asset transfers)
pub const BASE_GAS_FOR_CLAIM: Gas = Gas(15_000_000_000_000); // 7.5 TGas

/// For pessimistic allowance calculations:
/// Lowest amount of Gas a CCC can use 
pub const GAS_PER_CCC: Gas = Gas(5_000_000_000_000); // 5 TGas
/// How much Gas it costs to spin up the receipt
pub const RECEIPT_GAS_COST: Gas = Gas(2_500_000_000_000); // 2.5 TGas

/// Which specific use is something being acted on. This is not zero indexed (i.e the first use is 1)
pub type UseNumber = u16;
/// The ID of a given asset such as FTs, NFTs, or Function Calls.
pub type AssetId = String;
/// The ID for a given drop (this is the unique identifier for the drop and is how it will be referenced)
pub type DropId = String;
/// Drop Metadata should be a string which can be JSON or anything the users want.
pub type DropMetadata = String;