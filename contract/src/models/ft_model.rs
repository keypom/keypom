use crate::*;

/// Keep track fungible token data for an access key. This is stored on the contract
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FTData {
    pub contract_id: AccountId,
    pub sender_id: AccountId,
    pub balance_per_use: U128,
    pub ft_storage: U128,
}