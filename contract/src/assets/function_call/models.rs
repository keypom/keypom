use crate::*;

/// Keep track of info for the method_name to be called
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
    pub attached_deposit: U128,
    /// How much gas to attach to this method call.
    pub attached_gas: Gas,

    /// Keypom Args struct to be sent to external contracts
    pub keypom_args: Option<KeypomInjectedArgs>,
    /// If set to true, the claiming account ID will be the receiver ID of the method call.
    /// Ths receiver must be a valid account and non-malicious (cannot be set to the keypom contract)
    pub receiver_to_claimer: Option<bool>,
    /// What permissions does the user have when providing custom arguments to the function call?
    /// By default, the user cannot provide any custom arguments
    pub user_args_rule: Option<UserArgsRule>,
}

/// Keypom Args struct to be sent to external contracts
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

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, Debug)]
#[borsh(crate = "near_sdk::borsh")]
#[serde(crate = "near_sdk::serde")]
/// When a user provides arguments for FC drops in `claim` or `create_account_and_claim`, what behaviour is expected?
/// For `AllUser`, any arguments provided by the user will completely overwrite any previous args provided by the drop creator.
/// For `FunderPreferred`, any arguments provided by the user will be concatenated with the arguments provided by the drop creator. If there are any duplicate args, the drop funder's arguments will be used.
/// For `UserPreferred`, any arguments provided by the user will be concatenated with the arguments provided by the drop creator, but if there are any duplicate keys, the user's arguments will overwrite the drop funder's.
pub enum UserArgsRule {
    AllUser,
    FunderPreferred,
    UserPreferred,
}

