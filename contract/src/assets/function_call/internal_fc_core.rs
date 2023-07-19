use crate::*;

/// Keep track of info for the method_name to be called
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MethodData {
    /// Contract that will be called
    pub receiver_id: AccountId,
    /// Method to call on receiver_id contract
    pub method_name: String,
    /// Arguments to pass in (stringified JSON)
    pub args: String,
    /// Amount of yoctoNEAR to attach along with the call
    pub attached_deposit: U128,
    /// How much gas to attach to this method call. If none, all the gas is split between the parallel method calls in a given claim.
    /// If this is specified, the key can ONLY be used to call `claim` and no `deposit_per_use` can be specified. This leads the key to act like a method calling proxy instead of a linkrop.
    pub attached_gas: Gas,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PanicOnDefault)]
#[serde(crate = "near_sdk::serde")]
pub struct FCData {
    pub methods: Vec<MethodData>
}

impl FCData {
    /// Initialize a new set of FT data. The available balance is initialize to 0 at the start
    pub fn new(methods: Vec<MethodData>) -> Self {
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
    pub fn get_required_gas_for_claim(&self) -> Gas {
        // Loop through all the methods, tally up their attached gas and then:
        // Total Method Gas + MIN_BASE_GAS_FOR_ONE_CCC * num_methods + GAS_FOR_FC_CLAIM_LOGIC
        let mut total_gas = Gas(0);
        for method in self.methods.iter() {
            total_gas += method.attached_gas;
        }
        total_gas += MIN_BASE_GAS_FOR_ONE_CCC * self.methods.len() as u64;
        total_gas += GAS_FOR_FC_CLAIM_LOGIC;

        total_gas
    }
}