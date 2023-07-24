use crate::*;

/// An event log to capture whenever a new drop is created
///
/// Arguments
/// * `funder_id`: "account.near"
/// * `drop_id`: "my-drop123"
/// * `uses_per_key`: 1500
/// 
/// * `metadata`: optional metadata for the drop
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct DropCreationLog {
    pub funder_id: String,
    pub drop_id: String,
    pub uses_per_key: u32,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

/// An event log to capture whenever a drop is deleted
///
/// Arguments
/// * `funder_id`: "account.near"
/// * `drop_id`: "my-drop123"
/// 
/// * `metadata`: optional metadata for the drop
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct DropDeletionLog {
    pub funder_id: String,
    pub drop_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

/// An event log to capture whenever a key is added to or deleted from a drop
///
/// Arguments
/// * `owner_id`: "account.near"
/// * `drop_id`: "my-drop123"
/// * `public_key`: "ed25519:8WWpRpZswPHoJn5THFdBaFdbVeSjtSrQmEcoUeaNdkKy"
/// 
/// * `metadata`: optional metadata for the key
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct AddOrDeleteKeyLog {
    pub owner_id: String,
    pub drop_id: String,
    pub public_key: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

/// An event log to capture whenever a key is used to call `claim`
///
/// Arguments
/// * `account_id`: "account.near"
/// * `drop_id`: "my-drop123"
/// * `public_key`: "ed25519:8WWpRpZswPHoJn5THFdBaFdbVeSjtSrQmEcoUeaNdkKy"
/// * `assets`: list of assets that were claimed
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ClaimLog {
    pub account_id: String,
    pub drop_id: String,
    pub public_key: String,

    pub assets: Vec<Option<ExtAsset>>,
}

/// An event log to capture whenever a key is used to call `claim`
///
/// Arguments
/// * `account_id`: "account.near"
/// * `drop_id`: "my-drop123"
/// * `public_key`: "ed25519:8WWpRpZswPHoJn5THFdBaFdbVeSjtSrQmEcoUeaNdkKy"
/// * `assets`: list of assets that were claimed
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct CreateAccountAndClaimLog {
    pub new_account_id: String,
    pub new_public_key: String,
    pub public_key: String,
    pub drop_id: String,

    pub assets: Vec<Option<ExtAsset>>,
}