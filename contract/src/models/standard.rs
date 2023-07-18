use crate::*;

/// Information about a specific public key.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
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
   
   pub uses_remaining: UseNumber
}

/// Data outlining Fungible Tokens that should be sent to the claiming account
/// (either new or existing) when a key is successfully used.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FTListData {
    /// The number of tokens to transfer, wrapped in quotes and treated
    /// like a string, although the number will be stored as an unsigned integer
    /// with 128 bits.
    pub amount: String,

    /// The valid NEAR account indicating the Fungible Token contract.
    pub contract_id: String
}


/// Data outlining a specific Non-Fungible Token that should be sent to the claiming account
/// (either new or existing) when a key is successfully used.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NFTListData {
    /// the id of the token to transfer
    pub token_id: String,

    /// The valid NEAR account indicating the Non-Fungible Token contract.
    pub contract_id: String
}


