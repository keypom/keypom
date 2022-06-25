use near_sdk::{Balance, require};

use crate::{*, helpers::hash_account_id};

#[near_bindgen]
impl DropZone {
    /*
        user has created a bunch of keypairs and passed in the public keys and attached some deposit.
        this will store the account data and allow that keys to call claim and create_account_and_claim
        on this contract.

        The balance is the amount of $NEAR the sender wants each linkdrop to contain.
    */
    #[payable]
    pub fn create_drop(
        &mut self, 
        public_keys: Vec<PublicKey>, 
        balance: U128, 
        ft_data: Option<FTData>,
        nft_data: Option<NFTData>,
        fc_data: Option<FCData>
    ) {
        // Ensure the user has only specified one type of callback data
        let num_cbs_specified = if ft_data.is_some() {1} else {0} + if nft_data.is_some() {1} else {0} + if fc_data.is_some() {1} else {0};
        require!(num_cbs_specified <= 1, "You cannot specify more than one callback data");

        // Ensure the balance for each drop is larger than the minimum
        require!(
            balance.0 >= NEW_ACCOUNT_BASE,
            "cannot have a desired account balance less than the absolute minimum for creating an account"
        );

        // Funder is the predecessor
        let funder_id = env::predecessor_account_id();
        let len = public_keys.len() as u128;
        let drop_id = self.nonce;
        let attached_deposit = env::attached_deposit();

        // Pessimistically measure storage
        let initial_storage = env::storage_usage();

        let mut key_set: UnorderedSet<PublicKey> = UnorderedSet::new(StorageKey::DropsForFunderInner {
            //we get a new unique prefix for the collection
            account_id_hash: hash_account_id(&format!("{}{}", self.nonce, funder_id)),
        });

        // Add this drop ID to the funder's set of drops
        self.internal_add_drop_to_funder(&env::predecessor_account_id(), &drop_id);

        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in public_keys.clone() {
            key_set.insert(&pk);
            self.drop_id_for_pk.insert(&pk, &drop_id);
        }

        let mut drop = DropType { 
            funder_id: env::predecessor_account_id(), 
            balance, 
            pks: key_set,
            ft_data: ft_data.clone(), 
            nft_data: nft_data.clone(),
            fc_data: fc_data.clone(),
            storage_used_per_key: U128(u128::max_value()),
            keys_registered: 0
        };

        // Add drop type with largest possible storage used and keys registered for now.
        self.drop_type_for_id.insert(
            &drop_id, 
            &drop
        );

        // TODO: add storage for access keys * num of public keys
        // Calculate the storage being used for the entire drop and add it to the drop type.
        let final_storage = env::storage_usage();
        let total_required_storage = Balance::from(final_storage - initial_storage) * env::storage_byte_cost();

        // Insert the drop back with the storage
        drop.storage_used_per_key = U128(total_required_storage / len);
        self.drop_type_for_id.insert(
            &drop_id, 
            &drop
        );

        // Increment the drop ID nonce
        self.nonce += 1;

        let required_deposit = total_required_storage + (ACCESS_KEY_ALLOWANCE + balance.0 + if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0}) * len;
        env::log_str(&format!(
            "Attached Deposit: {}, 
            Required Deposit: {}, 
            Total Required Storage: {}, 
            ACCESS_KEY_STORAGE: {},
            ACCESS_KEY_ALLOWANCE: {}, 
            Linkdrop Balance: {}, 
            total function call deposits (if applicable): {}, 
            length: {}", 
            yocto_to_near(attached_deposit), 
            yocto_to_near(required_deposit),
            yocto_to_near(total_required_storage), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(ACCESS_KEY_ALLOWANCE), 
            yocto_to_near(balance.0), 
            yocto_to_near(if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0}), 
            len)
        );
        /*
            Ensure the attached deposit can cover: 
        */ 
        require!(attached_deposit >= required_deposit, "Not enough deposit");

        
        // Create a new promise batch to create all the access keys
        let current_account_id = env::current_account_id();
        let promise = env::promise_batch_create(&current_account_id);
        
        // Loop through each public key and create the access keys
        for pk in public_keys.clone() {
            // Must assert in the loop so no access keys are made?
            env::promise_batch_action_add_key_with_function_call(
                promise, 
                &pk, 
                0, 
                ACCESS_KEY_ALLOWANCE, 
                &current_account_id, 
                ACCESS_KEY_METHOD_NAMES
            );
        }

        env::promise_return(promise);

        // Check if user will attach fungible tokens
        if ft_data.is_some() {

            /*
                Get the storage required by the FT contract and ensure the user has attached enough
                deposit to cover the storage and perform refunds if they overpayed.
            */ 

            ext_ft_contract::ext(ft_data.unwrap().ft_contract)
                // Call storage balance bounds with exactly this amount of GAS. No unspent GAS will be added on top.
                .with_static_gas(GAS_FOR_STORAGE_BALANCE_BOUNDS)
                .with_unused_gas_weight(0)
                .storage_balance_bounds()
            .then(
                Self::ext(env::current_account_id())
                    // Resolve the promise with the attached deposit and the min GAS. All unspent GAS will be added to this call.
                    .with_static_gas(MIN_GAS_FOR_RESOLVE_STORAGE_CHECK)
                    .with_attached_deposit(attached_deposit)
                    .resolve_storage_check(
                        public_keys,
                        drop_id
                    )
            );
        } else if attached_deposit > required_deposit {
            env::log_str(&format!("Refunding User for: {}", yocto_to_near(attached_deposit - required_deposit)));    
            // If the user overpaid for the desired linkdrop balances, refund them.
            Promise::new(env::predecessor_account_id()).transfer(attached_deposit - required_deposit);
        }
    }
}