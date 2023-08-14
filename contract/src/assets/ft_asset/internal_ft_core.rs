use crate::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Clone)]
pub struct InternalFTData {
    /// Account ID of the token contract
    pub contract_id: AccountId,
    /// Number of tokens that are available for transfer.
    /// To make tokens available for transfer, you must send them via `ft_transfer_call`.
    pub balance_avail: Balance,
    /// How much it costs to register a new user on the FT contract
    pub registration_cost: Balance
}

// Implement a custom serialization that converts both `balance_avail` and `registration_cost` to a `U128` for the frontend
impl Serialize for InternalFTData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let mut state = serializer.serialize_struct("InternalFTData", 3)?;
        state.serialize_field("contract_id", &self.contract_id)?;
        state.serialize_field("balance_avail", &U128(self.balance_avail))?;
        state.serialize_field("registration_cost", &U128(self.registration_cost))?;
        state.end()
    }
}

impl InternalFTData {
    /// Initialize a new set of FT data. The available balance is initialize to 0 at the start
    pub fn new(contract_id: AccountId, registration_cost: Balance) -> Self {
        Self {
            contract_id,
            balance_avail: 0,
            registration_cost
        }
    }

    /// Query whether or not enough fungible tokens have been deposited to cover the claim
    pub fn enough_balance(&self, amount: &Balance) -> bool {
        &self.balance_avail >= amount
    }

    /// Get the asset ID for this asset. In the case of Fungible Tokens, the ID is the contract
    pub fn get_asset_id(&self) -> AssetId {
        self.contract_id.to_string()
    }

    /// Query how much gas is required for a single claim
    pub fn get_required_asset_gas(&self) -> Gas {
        MIN_GAS_FOR_FT_TRANSFER + MIN_GAS_FOR_STORAGE_DEPOSIT
    }

    /// Query how much gas is required for a single claim
    pub fn get_total_required_gas(&self) -> Gas {
        GAS_FOR_FT_CLAIM_LOGIC + MIN_GAS_FOR_FT_TRANSFER + MIN_GAS_FOR_STORAGE_DEPOSIT
    }
}