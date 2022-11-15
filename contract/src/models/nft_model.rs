use crate::*;
use near_sdk::collections::Vector;

/// Keep track of nft data. This is stored on the contract
#[derive(BorshDeserialize, BorshSerialize)]
pub struct NFTData {
    pub sender_id: AccountId,
    pub contract_id: AccountId,
    pub token_ids: Vector<String>,
}