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
    // Internal method for transfer NFTs.
    pub(crate) fn internal_fc_execute(
        &mut self,
        fc_data: FCData,
        amount_to_refund: u128,
        account_id: AccountId,
    ) {
        /*
            Function Calls
        */
        let mut final_args = fc_data.args.clone();

        // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
        if let Some(account_field) = fc_data.claimed_account_field {
            final_args.insert_str(final_args.len()-1, &format!(",\"{}\":\"{}\"", account_field, account_id));
            env::log_str(&format!("Adding claimed account ID to specified field: {:?} in args: {:?}", account_field, fc_data.args));
        }
    
        env::log_str(&format!(
            "Attaching Total: {:?} Deposit: {:?} Should Refund?: {:?} Amount To Refund: {:?} With args: {:?}", 
            yocto_to_near(fc_data.deposit.0 + if fc_data.refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}), 
            yocto_to_near(fc_data.deposit.0), fc_data.refund_to_deposit.unwrap_or(false), yocto_to_near(amount_to_refund), 
            final_args
        ));

        // Call function with the min GAS and deposit. all unspent GAS will be added on top
        Promise::new(fc_data.receiver).function_call_weight(
            fc_data.method, 
            final_args.as_bytes().to_vec(), 
            // The claim is successful so attach the amount to refund to the deposit instead of refunding the funder.
            fc_data.deposit.0 + if fc_data.refund_to_deposit.unwrap_or(false) {amount_to_refund} else {0}, 
            fc_data.gas_if_straight_execute.unwrap_or(Gas(0)),
            GasWeight(1)
        );
    }
}