use near_sdk::serde::Serialize;

use crate::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct InternalFTData {
    /// Account ID of the token contract
    pub contract_id: AccountId,
    /// Number of tokens that are available for transfer.
    /// To make tokens available for transfer, you must send them via `ft_transfer_call`.
    pub balance_avail: u128,
    /// How much it costs to register a new user on the FT contract
    pub registration_cost: u128
}

impl InternalFTData {
    /// Initialize a new set of FT data. The `available` field is set to 0 to start until tokens are transferred.
    pub fn new(contract_id: AccountId, registration_cost: u128) -> Self {
        Self {
            contract_id,
            balance_avail: 0,
            registration_cost
        }
    }
}