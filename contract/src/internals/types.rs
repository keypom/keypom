/// Which specific use is something being acted on. This is not zero indexed (i.e the first use is 1)
pub type UseNumber = u32;
/// The ID of a given asset such as FTs, NFTs, or Function Calls.
pub type AssetId = String;
/// The ID for a given drop (this is the unique identifier for the drop and is how it will be referenced)
pub type DropId = String;
/// Drop Metadata should be a string which can be JSON or anything the users want.
pub type DropMetadata = String;
/// ID for NFTs that have been sent to the Keypom contract as part of NFT assets
pub type TokenId = String;

/// Optionally, the user can pass in a set of asset specific FC args
pub type UserProvidedFCArgs = Vec<AssetSpecificFCArgs>;
/// For each FC asset, there can be a set of user arguments for each method. Each method doesn't necessarily need to have user arguments.
pub type AssetSpecificFCArgs = Option<Vec<Option<String>>>;

