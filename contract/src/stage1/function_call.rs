use near_sdk::GasWeight;

use crate::*;

/// Keep track of info for the method to be called
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MethodData {
    // Contract that will be called
    pub receiver: AccountId,
    // Method to call on receiver contract
    pub method: String,
    // Arguments to pass in (stringified JSON)
    pub args: String,
    // Amount of yoctoNEAR to attach along with the call
    pub deposit: U128,
}

/// Keep track of optional configurations for the FC data
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCConfig {
    // Should the refund that normally goes to the funder be attached alongside the deposit?
    // If Some(true), refund will go to the deposit. Else, it will be refunded to the funder.
    pub refund_to_deposit: Option<bool>,

    // Specifies what field the claiming account should go in when calling the function
    // If None, this isn't attached to the args
    pub claimed_account_field: Option<String>,

    // Specifies what field the drop ID should go in when calling the function.
    // If Some(String), attach drop ID to args. Else, don't attach.
    pub attached_drop_id_field: Option<String>,

    // How much GAS should be attached to the function call if it's a straight execute. Cannot be greater than ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE (90 TGas).
    // This makes it so the keys can only call `claim`
    pub gas_if_claim_only: Option<Gas>,
}

/// Keep track of nft data
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    // Vector of optional method data. If None, no method is called.
    // Drops with more than 1 claim can call different functions each time if Some.
    // If only 1 method is passed in for multiple claims, that method is used for every claim.
    pub method_data: Vec<Option<MethodData>>,

    // Config for the FC data. If None, all default values are used.
    pub config: Option<FCConfig>,
}

#[near_bindgen]
impl DropZone {
    // Internal method for transfer NFTs.
    pub(crate) fn internal_fc_execute(
        &mut self,
        method_data: &MethodData,
        fc_config: Option<FCConfig>,
        amount_to_refund: u128,
        account_id: AccountId,
        drop_id: DropId,
    ) {
        /*
            Function Calls
        */
        let mut final_args = method_data.args.clone();

        // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
        if let Some(account_field) = fc_config.clone().and_then(|c| c.claimed_account_field) {
            final_args.insert_str(
                final_args.len() - 1,
                &format!(",\"{}\":\"{}\"", account_field, account_id),
            );
            near_sdk::log!(
                "Adding claimed account ID to specified field: {:?} in args: {:?}",
                account_field,
                method_data.args
            );
        }

        // Add the account ID that claimed the linkdrop as part of the args to the function call in the key specified by the user
        if let Some(drop_id_field) = fc_config.clone().and_then(|c| c.attached_drop_id_field) {
            final_args.insert_str(
                final_args.len() - 1,
                &format!(",\"{}\":\"{}\"", drop_id_field, drop_id),
            );
            near_sdk::log!("Adding drop ID to args {:?}", drop_id,);
        }

        near_sdk::log!(
            "Attaching Total: {:?} Deposit: {:?} Should Refund?: {:?} Amount To Refund: {:?} With args: {:?}", 
            yocto_to_near(method_data.deposit.0 + if fc_config.clone().and_then(|c| c.refund_to_deposit).unwrap_or(false) {amount_to_refund} else {0}),
            yocto_to_near(method_data.deposit.0), fc_config.clone().and_then(|c| c.refund_to_deposit).unwrap_or(false), yocto_to_near(amount_to_refund),
            final_args
        );

        // Call function with the min GAS and deposit. all unspent GAS will be added on top
        Promise::new(method_data.receiver.clone()).function_call_weight(
            method_data.method.clone(),
            final_args.as_bytes().to_vec(),
            // The claim is successful so attach the amount to refund to the deposit instead of refunding the funder.
            method_data.deposit.0
                + if fc_config
                    .clone()
                    .and_then(|c| c.refund_to_deposit)
                    .unwrap_or(false)
                {
                    amount_to_refund
                } else {
                    0
                },
            fc_config
                .and_then(|c| c.gas_if_claim_only)
                .unwrap_or(Gas(0)),
            GasWeight(1),
        );
    }
}
