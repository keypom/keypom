use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::ser::SerializeStruct;
use near_sdk::serde::{Deserialize, Serialize, Serializer};
use near_sdk::serde_json::json;
use near_sdk::{
    env, near_bindgen, require, AccountId, Allowance, BorshStorageKey, CryptoHash, Gas, GasWeight,
    NearToken, PanicOnDefault, Promise, PromiseOrValue, PublicKey,
};
use std::collections::{HashMap, HashSet};

use near_workspaces::types::{KeyType, SecretKey};

pub struct SignatureMeta {
    pub message: String,
    pub secret_key: SecretKey,
}

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
pub type UserProvidedFCArgs = Option<Vec<AssetSpecificFCArgs>>;
/// For each FC asset, there can be a set of user arguments for each method. Each method doesn't necessarily need to have user arguments.
pub type AssetSpecificFCArgs = Option<Vec<Option<String>>>;

#[derive(BorshDeserialize, Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct ExtDrop {
    /// ID for this specific drop
    pub drop_id: DropId,
    /// Account ID who funded / owns the rights to this specific drop
    pub funder_id: String,
    /// What is the maximum number of uses a given key can have in the drop?
    pub max_key_uses: UseNumber,

    pub asset_data: Vec<ExtAssetDataForUses>,

    pub nft_asset_data: Vec<InternalNFTData>,
    pub ft_asset_data: Vec<InternalFTData>,

    /// Keep track of different configuration options for all the uses of a key in a given drop
    pub drop_config: Option<DropConfig>,

    /// Keep track of the next nonce to give out to a key
    pub next_key_id: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct ExtKeyInfo {
    /// How much Gas should be attached when the key is used to call `claim` or `create_account_and_claim`.
    /// It is up to the smart contract developer to calculate the required gas (which can be done either automatically on the contract or on the client-side).
    pub required_gas: String,

    /// yoctoNEAR$ amount that will be sent to the account that claims the linkdrop (either new or existing)
    /// when the key is successfully used.
    pub yoctonear: U128,

    /// If using the FT standard extension, a set of FTData can be linked to the public key
    /// indicating that all those assets will be sent to the account that claims the linkdrop (either new or
    /// existing) when the key is successfully used.
    pub ft_list: Vec<FTListData>,

    /// If using the NFT standard extension, a set of NFTData can be linked to the public key
    /// indicating that all those assets will be sent to the account that claims the linkdrop (either new or
    /// existing) when the key is successfully used.
    pub nft_list: Vec<NFTListData>,

    /* CUSTOM */
    pub drop_id: DropId,
    pub pub_key: PublicKey,
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub fc_list: Vec<FCData>,

    pub uses_remaining: UseNumber,

    pub message_nonce: u32,
}

#[derive(
    BorshSerialize, BorshDeserialize, Serialize, Deserialize, PanicOnDefault, Clone, Debug,
)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    pub methods: Vec<MethodData>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct FTListData {
    /// The number of tokens to transfer, wrapped in quotes and treated
    /// like a string, although the number will be stored as an unsigned integer
    /// with 128 bits.
    pub amount: String,

    /// The valid NEAR account indicating the Fungible Token contract.
    pub contract_id: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct NFTListData {
    /// the id of the token to transfer
    pub token_id: String,

    /// The valid NEAR account indicating the Non-Fungible Token contract.
    pub contract_id: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct DropConfig {
    /// Metadata for the given drop represented as a string. Most often, this will be JSON stringified.
    pub metadata: Option<DropMetadata>,
    /// Configurations for all the NFT keys in this drop. This contains info about royalties and metadata
    /// That each key will inherit
    pub nft_keys_config: Option<NFTKeyConfigurations>,

    /// Which users can add keys to the drop. The public sale config was moved out of the Keypom contract
    /// And now should be deployed on its own proxy contract that in turn performs any necessary sale logic
    /// And then fires a cross contract call to the Keypom contract to add keys
    pub add_key_allowlist: Option<HashSet<String>>,

    /// Should the drop be automatically deleted when all the keys are used? This is defaulted to true and
    /// Must be overwritten
    pub delete_empty_drop: Option<bool>,

    /// How much extra allowance should be given to each key in the drop?
    /// This allows keys to be used for extra functionalities such as `nft_transfer`, `nft_approve`, etc.
    pub extra_allowance_per_key: Option<String>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct NFTKeyConfigurations {
    /// Metadata that all the NFTs in this drop will have
    pub token_metadata: Option<TokenMetadata>,

    /// Royalties that all NFTs in this drop will have
    pub royalties: Option<HashMap<String, u32>>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct TokenMetadata {
    pub title: Option<String>, // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
    pub description: Option<String>, // free-form description
    pub media: Option<String>, // URL to associated media, preferably to decentralized, content-addressed storage
    pub copies: Option<u64>, // number of copies of this set of metadata in existence when token was minted.
    pub issued_at: Option<u64>, // When token was issued or minted, Unix epoch in milliseconds
    pub expires_at: Option<u64>, // When token expires, Unix epoch in milliseconds
    pub starts_at: Option<u64>, // When token starts being valid, Unix epoch in milliseconds
    pub updated_at: Option<u64>, // When token was last updated, Unix epoch in milliseconds
    pub extra: Option<String>, // anything extra the NFT wants to store on-chain. Can be stringified JSON.
    pub reference: Option<String>, // URL to an off-chain JSON file with more info.
}

#[derive(
    BorshSerialize, BorshDeserialize, PanicOnDefault, Debug, Serialize, Deserialize, Clone,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct InternalFTData {
    /// Account ID of the token contract
    pub contract_id: String,
    /// Number of tokens that are available for transfer.
    /// To make tokens available for transfer, you must send them via `ft_transfer_call`.
    pub balance_avail: u128,
    /// How much it costs to register a new user on the FT contract
    pub registration_cost: u128,
}

#[derive(
    BorshSerialize, BorshDeserialize, PanicOnDefault, Debug, Serialize, Deserialize, Clone,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct InternalNFTData {
    /// Account ID of the token contract
    pub contract_id: String,
    /// Token IDs that are available to be claimed and have been sent via `nft_transfer_call`
    pub token_ids: Vec<TokenId>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct ExtAssetDataForUses {
    /// How many uses does this asset data apply to?
    pub uses: UseNumber,
    /// Which assets should be present for these uses
    pub assets: Vec<Option<ExtAsset>>,
    /// Any configurations for this set of uses
    pub config: Option<UseConfig>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct UseConfig {
    /// Configurations related to how often keys can be used
    pub time: Option<TimeConfig>,

    /// Can the access key for this use call the claim method_name? Default to both method_name callable
    pub permissions: Option<ClaimPermissions>,

    /// When calling `create_account` on the root account, which keypom args should be attached to the payload.
    pub account_creation_keypom_args: Option<KeypomInjectedArgs>,

    /// Override the global root account that sub-accounts will have (near or testnet). This allows
    /// users to create specific drops that can create sub-accounts of a predefined root.
    /// For example, Fayyr could specify a root of `fayyr.near` By which all sub-accounts will then
    /// be `ACCOUNT.fayyr.near`
    pub root_account_id: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub enum ClaimPermissions {
    claim,
    create_account_and_claim,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct TimeConfig {
    /// Minimum block timestamp before keys can be used. If None, keys can be used immediately
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub start: Option<u64>,

    /// Block timestamp that keys must be before. If None, keys can be used indefinitely
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub end: Option<u64>,

    /// Time interval between each key use. If None, there is no delay between key uses.
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub throttle: Option<u64>,

    /// Interval of time after the `start_timestamp` that must pass before a key can be used.
    /// If multiple intervals pass, the key can be used multiple times. This has nothing to do
    /// With the throttle timestamp. It only pertains to the start timestamp and the current
    /// timestamp. The last_used timestamp is not taken into account.
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub interval: Option<u64>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
#[serde(untagged)]
pub enum ExtAsset {
    FTAsset(ExtFTData),
    NearAsset(ExtNEARData),
    NFTAsset(ExtNFTData),
    FCAsset(Vec<MethodData>),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct MethodData {
    /// Contract that will be called
    pub receiver_id: String,
    /// Method to call on receiver_id contract
    pub method_name: String,
    /// Arguments to pass in (stringified JSON)
    pub args: String,
    /// Amount of yoctoNEAR to attach along with the call
    pub attached_deposit: String,
    /// How much gas to attach to this method call.
    pub attached_gas: String,

    /// Keypom Args struct to be sent to external contracts
    pub keypom_args: Option<KeypomInjectedArgs>,
    /// If set to true, the claiming account ID will be the receiver ID of the method call.
    /// Ths receiver must be a valid account and non-malicious (cannot be set to the keypom contract)
    pub receiver_to_claimer: Option<bool>,
    /// What permissions does the user have when providing custom arguments to the function call?
    /// By default, the user cannot provide any custom arguments
    pub user_args_rule: Option<UserArgsRule>,
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub enum UserArgsRule {
    AllUser,
    FunderPreferred,
    UserPreferred,
}

#[derive(Serialize, Deserialize, Debug, BorshDeserialize, BorshSerialize, Clone)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct KeypomInjectedArgs {
    /// Specifies what field the claiming account ID should go in when calling the function
    /// If None, this isn't attached to the args
    pub account_id_field: Option<String>,
    /// Specifies what field the drop ID should go in when calling the function. To insert into nested objects, use periods to separate. For example, to insert into args.metadata.field, you would specify "metadata.field"
    /// If Some(String), attach drop ID to args. Else, don't attach.
    pub drop_id_field: Option<String>,
    /// Specifies what field the key ID should go in when calling the function. To insert into nested objects, use periods to separate. For example, to insert into args.metadata.field, you would specify "metadata.field"
    /// If Some(String), attach key ID to args. Else, don't attach.
    pub key_id_field: Option<String>,
    // Specifies what field the funder id should go in when calling the function. To insert into nested objects, use periods to separate. For example, to insert into args.metadata.field, you would specify "metadata.field"
    // If Some(string), attach the funder ID to the args. Else, don't attach.
    pub funder_id_field: Option<String>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct ExtNEARData {
    /// How much $NEAR should be transferred as part of the asset claim
    pub yoctonear: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
pub struct ExtFTData {
    /// The contract that the fungible tokens originate from
    pub ft_contract_id: String,
    /// How much $NEAR (in yocto) it costs to register a new user on the fungible token contract
    pub registration_cost: String,
    /// How many fungible tokens (in their smallest indivisible unit) should be transferred as part of the asset claim
    pub ft_amount: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct ExtNFTData {
    /// The contract that the non fungible tokens originate from
    pub nft_contract_id: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct ExtKeyData {
    /// What is the public key?
    pub public_key: String,
    /// A map outlining what the password should be for any given use.
    /// The password here should be a double hash and when claim is called,
    /// The user arguments are hashed and compared to the password here (i.e user passes in single hash)
    pub password_by_use: Option<HashMap<UseNumber, String>>,
    /// Metadata for the given key represented as a string. Most often, this will be JSON stringified.
    pub metadata: Option<String>,
    /// What account ID owns the given key (if any)
    pub key_owner: Option<String>,
}
