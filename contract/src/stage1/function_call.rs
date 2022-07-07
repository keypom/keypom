use near_sdk::GasWeight;

use crate::*;

/// Keep track of nft data 
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    // Contract that will be called
    pub receiver: AccountId,
    // Method to call on receiver contract
    pub method: String,
    // Arguments to pass in (stringified JSON)
    pub args: String,
    // Amount of yoctoNEAR to attach along with the call
    pub deposit: U128,
    // Should the refund that normally goes to the funder be attached alongside the deposit?
    pub refund_to_deposit: Option<bool>,
    // Specifies what field the claiming account should go in when calling the function
    pub claimed_account_field: Option<String>,
    // How much GAS should be attached to the function call if it's a straight execute. Cannot be greater than ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE (90 TGas).
    // This makes it so the keys can only call `claim`
    pub gas_if_straight_execute: Option<Gas>
}

#[near_bindgen]
impl DropZone {
    // Internal method for transfer NFTs. Whether the claim was successful or not is passed in
    pub(crate) fn internal_fc_execute(
        &mut self,
        claim_succeeded: bool,
        // Receiver of the function call
        receiver: AccountId,
        // Method to call on the contract
        method: String,
        // What args to pass in
        args: String,
        // What deposit should we attach
        deposit: U128,
        // Should we add the account ID as part of the args and what key should it live in
        claimed_account_field: Option<String>,
        // Should the refund be sent to the funder or attached to the deposit
        add_refund_to_deposit: Option<bool>,
        // How much GAS to attach. If None, default to 0 and attach all unused GAS
        gas: Option<Gas>,
        amount_to_refund: u128,
        account_id: AccountId,
    ) {
        /*
            Function Calls
        */
        // Only call the function if the claim was successful.
        if claim_succeeded {
            let mut final_args = args.clone();

            // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
            if let Some(account_field) = claimed_account_field {
                final_args.insert_str(final_args.len()-1, &format!(",\"{}\":\"{}\"", account_field, account_id));
                env::log_str(&format!("Adding claimed account ID to specified field: {:?} in args: {:?}", account_field, args));
            }
        
            env::log_str(&format!("Attaching Total: {:?} Deposit: {:?} Should Refund?: {:?} Amount To Refund: {:?} With args: {:?}", yocto_to_near(deposit.0 + if add_refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}), yocto_to_near(deposit.0), add_refund_to_deposit.unwrap_or(false), yocto_to_near(amount_to_refund), final_args));

            // Call function with the min GAS and deposit. all unspent GAS will be added on top
            Promise::new(receiver).function_call_weight(
                method, 
                final_args.as_bytes().to_vec(), 
                // The claim is successful so attach the amount to refund to the deposit instead of refunding the funder.
                deposit.0 + if add_refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}, 
                gas.unwrap_or(Gas(0)),
                GasWeight(1)
            );
        }
    }
}