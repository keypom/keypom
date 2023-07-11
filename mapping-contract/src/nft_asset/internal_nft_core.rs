use crate::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Debug, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct InternalNFTData {
    /// Account ID of the token contract
    pub contract_id: AccountId,
    /// Token IDs that are available to be claimed and have been sent via `nft_transfer_call`
    pub token_ids: Vec<TokenId>
}

impl InternalNFTData {
    /// Initialize a new set of FT data. The available balance is initialize to 0 at the start
    pub fn new(contract_id: AccountId) -> Self {
        Self {
            contract_id,
            token_ids: vec![]
        }
    }

    /// Query whether or not there is at least 1 NFT to transfer
    pub fn is_empty(&self) -> bool {
        self.token_ids.is_empty()
    }

    /// Query whether or not there is at least 1 NFT to transfer
    pub fn get_next_token_id(&self) -> Option<TokenId> {
        self.token_ids.last().cloned()
    }

    /// Query how much gas is required for a single claim
    pub fn get_required_gas_for_claim(&self) -> Gas {
        GAS_FOR_NFT_CLAIM_LOGIC + MIN_GAS_FOR_NFT_TRANSFER
    }
}