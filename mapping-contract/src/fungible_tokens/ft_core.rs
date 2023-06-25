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
    /// Initialize a new set of FT data. The available balance is initialize to 0 at the start
    pub fn new(contract_id: AccountId, registration_cost: u128) -> Self {
        Self {
            contract_id,
            balance_avail: 0,
            registration_cost
        }
    }

    /// Return the amount of NEAR required to have been deposited for 1 claim
    /// This is a standard method that should be implemented by each struct and should
    /// Be called by each data type before each claim
    pub fn required_near_for_claim(&self) -> u128 {
        self.registration_cost
    }

    /// Query whether or not enough fungible tokens have been deposited to cover the claim
    pub fn enough_balance(&self, amount: &u128) -> bool {
        &self.balance_avail >= amount
    }

    /// Get the asset ID for this asset. In the case of Fungible Tokens, the ID is the contract
    pub fn get_asset_id(&self) -> AssetId {
        self.contract_id.to_string()
    }
}