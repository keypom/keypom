use std::fmt;

use crate::*;

/// Enum that represents the data type of the EventLog.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
#[serde(crate = "near_sdk::serde")]
#[non_exhaustive]
pub enum EventLogVariant {
    // Keypom specific events
    DropCreation(DropCreationLog),
    DropDeletion(DropDeletionLog),
    AddKey(Vec<AddOrDeleteKeyLog>),
    DeleteKey(Vec<AddOrDeleteKeyLog>),
    Claim(ClaimLog),
    CreateAccountAndClaim(CreateAccountAndClaimLog),

    // NFT specific events
    NftMint(Vec<NftMintLog>),
    NftTransfer(Vec<NftTransferLog>),
    
}

/// Interface to capture data about an event
///
/// Arguments:
/// * `standard`: name of standard e.g. kpom1
/// * `version`: e.g. 1.0.0
/// * `event`: associate event data
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct EventLog {
    pub standard: String,
    pub version: String,

    // `flatten` to not have "event": {<EventLogVariant>} in the JSON, just have the contents of {<EventLogVariant>}.
    #[serde(flatten)]
    pub event: EventLogVariant,
}

impl fmt::Display for EventLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "EVENT_JSON:{}",
            &serde_json::to_string(self).map_err(|_| fmt::Error)?
        ))
    }
}