use crate::*;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PanicOnDefault, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    pub methods: Vec<MethodData>
}

impl FCData {
    /// Initialize a new instance of function call data.
    /// All checks such as prohibited methods and valid receivers are done here.
    pub fn new(methods: Vec<MethodData>) -> Self {
        require!(methods.len() > 0, "Must have at least 1 method in FC assets");

        for method in methods.iter() {
            // Check if the method is prohibited
            require!(!DEFAULT_PROHIBITED_FC_METHODS.contains(&method.method_name.as_str()), format!("Method {} is prohibited from being called in an FC drop", method.method_name));

            // Check if the receiver is valid
            require!(method.receiver_id != env::current_account_id().to_string(), "Receiver ID cannot be current Keypom contract.");
        }
        
        Self {
            methods
        }
    }

    /// Query how much $NEAR should be refunded for 1 claim
    pub fn get_yocto_refund_amount(&self) -> Balance {
        // Loop through all the methods, tally up their attached deposits and return that
        let mut total_refund = 0;
        for method in self.methods.iter() {
            total_refund += method.attached_deposit.0;
        }

        total_refund
    }

    /// Query how much gas is required for a single claim
    pub fn get_required_asset_gas(&self) -> Gas {
        // Loop through all the methods, tally up their attached gas and then:
        // Total Method Gas + MIN_BASE_GAS_FOR_ONE_CCC * num_methods + GAS_FOR_FC_CLAIM_LOGIC + num_methods + the length of the arguments
        let mut total_gas = Gas(0);
        for method in self.methods.iter() {
            total_gas += method.attached_gas;
            total_gas += MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP + GAS_FOR_FC_CLAIM_LOGIC;
        }

        total_gas
    }

    /// Query how much gas is required for a single claim
    pub fn get_total_required_gas(&self) -> Gas {
        // Loop through all the methods, tally up their attached gas and then:
        // Total Method Gas + MIN_BASE_GAS_FOR_ONE_CCC * num_methods + GAS_FOR_FC_CLAIM_LOGIC + num_methods + the length of the arguments
        let mut total_gas = Gas(0);
        for method in self.methods.iter() {
            let arg_len = method.args.len() as u64;
            total_gas += GAS_PER_ARG_LENGTH * arg_len;
            total_gas += method.attached_gas;
            total_gas += MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP + GAS_FOR_FC_CLAIM_LOGIC;
        }

        total_gas
    }
}