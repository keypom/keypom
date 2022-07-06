use crate::*;
use near_sdk::{Balance, require};

pub type DropId = u128;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum DropType {
    Simple,
    NFT(NFTData),
    FT(FTData),
    FC(FCData),
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize)]
pub struct KeyConfiguration {
    // Number of uses for each key in the drop. If None, unlimited uses
    pub num_uses: Option<u64>,

    // Minimum block timestamp that keys can be used. If None, keys can be used immediately
    pub start_timestamp: Option<u64>,

    // How often can a key be used 
    pub usage_interval: Option<u64>,
}

/// Keep track of specific data related to an access key. This allows us to optionally refund funders later. 
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Drop {
    // Funder of this specific drop
    pub funder_id: AccountId,
    // Set of public keys associated with this drop
    pub pks: UnorderedSet<PublicKey>,

    // Balance for all keys of this drop. Can be 0 if specified.
    pub balance: U128,

    // Every drop must have a type
    pub drop_type: DropType,

}

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
        nft_data: Option<JsonNFTData>,
        fc_data: Option<FCData>
    ) -> DropId {
        // Ensure the user has only specified one type of callback data
        let num_cbs_specified = ft_data.is_some() as u8 + nft_data.is_some() as u8 + fc_data.is_some() as u8;        
        env::log_str(&format!("Num CBs {}", num_cbs_specified));
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

        // Get the current balance of the funder. 
        let mut current_user_balance = self.user_balances.get(&funder_id).expect("No user balance found");
        env::log_str(&format!("Cur User balance {}", yocto_to_near(current_user_balance)));
        
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();
        let mut key_set: UnorderedSet<PublicKey> = UnorderedSet::new(StorageKey::DropIdsForFunderInner {
            //we get a new unique prefix for the collection
            account_id_hash: hash_account_id(&format!("{}{}", self.nonce, funder_id)),
        });

        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in &public_keys {
            key_set.insert(pk);
            require!(self.drop_id_for_pk.insert(pk, &drop_id).is_none(), "Keys cannot belong to another drop");
        }

        // Create drop object 
        let mut drop = Drop { 
            funder_id: env::predecessor_account_id(), 
            balance, 
            pks: key_set,
            ft_data: ft_data.clone(),
            nft_data: None, // Defaulting to None
            fc_data: fc_data.clone(),
            keys_registered: 0
        };

        // For NFT drops, measure the storage for adding the longest token ID
        let mut storage_per_longest = 0;

        // If NFT data was provided, we need to build the set of token IDs
        if nft_data.is_some() {
            let JsonNFTData{nft_sender, nft_contract, longest_token_id, storage_for_longest: _} = nft_data.unwrap();

            // Create the token ID set and insert the longest token ID
            let token_ids = UnorderedSet::new(StorageKey::TokenIdsForDrop {
                //we get a new unique prefix for the collection
                account_id_hash: hash_account_id(&format!("nft-{}{}", self.nonce, funder_id)),
            });

            // Create the NFT data
            let actual_nft_data = NFTData {
                nft_sender,
                nft_contract,
                longest_token_id: longest_token_id.clone(),
                storage_for_longest: u128::MAX,
                token_ids: Some(token_ids)
            };

            drop.nft_data = Some(actual_nft_data);
            // Add the drop with the empty token IDs
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
            
            // Measure how much storage it costs to insert the 1 longest token ID
            let initial_nft_storage_one = env::storage_usage();
            // Now that the drop has been added, insert the longest token ID and measure storage
            if let Some(data) = &mut drop.nft_data {
                if let Some(ids) = &mut data.token_ids {
                    ids.insert(&longest_token_id);
                }
            }
            // Add drop with the longest possible token ID and max storage
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
            let final_nft_storage_one = env::storage_usage();
            env::log_str(&format!("i1: {} f1: {}", initial_nft_storage_one, final_nft_storage_one));

            // Measure the storage per single longest token ID
            storage_per_longest = Balance::from(final_nft_storage_one - initial_nft_storage_one);
            env::log_str(&format!("TOKS BEFORE {:?}", self.get_token_ids_for_drop(self.nonce, None, None)));

            // Clear the token IDs so it's an empty set and put the storage in the drop's nft data
            if let Some(data) = &mut drop.nft_data {
                if let Some(ids) = &mut data.token_ids {
                    ids.clear();
                }

                data.storage_for_longest = storage_per_longest;
            }
        }

        // Add this drop ID to the funder's set of drops
        self.internal_add_drop_to_funder(&env::predecessor_account_id(), &drop_id);

        // Add drop with largest possible storage used and keys registered for now.
        self.drop_for_id.insert(
            &drop_id, 
            &drop
        );

        // TODO: add storage for access keys * num of public keys
        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_required_storage = (Balance::from(final_storage - initial_storage) + storage_per_longest) * env::storage_byte_cost();
        env::log_str(&format!("Total required storage Yocto {}", total_required_storage));

        // Increment the drop ID nonce
        self.nonce += 1;

        let required_deposit = self.drop_fee + total_required_storage + (self.key_fee + ACCESS_KEY_STORAGE + ACCESS_KEY_ALLOWANCE + balance.0 + if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0} + storage_per_longest * env::storage_byte_cost()) * len;
        env::log_str(&format!(
            "Current balance: {}, 
            Required Deposit: {}, 
            Drop Fee: {}, 
            Total Required Storage: {}, 
            Key Fee: {}, 
            ACCESS_KEY_STORAGE: {},
            ACCESS_KEY_ALLOWANCE: {}, 
            Linkdrop Balance: {}, 
            total function call deposits (if applicable): {}, 
            Storage for longest token ID (if applicable): {},
            length: {}", 
            yocto_to_near(current_user_balance), 
            yocto_to_near(required_deposit),
            yocto_to_near(self.drop_fee),
            yocto_to_near(total_required_storage), 
            yocto_to_near(self.key_fee),
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(ACCESS_KEY_ALLOWANCE), 
            yocto_to_near(balance.0), 
            yocto_to_near(if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0}), 
            yocto_to_near(storage_per_longest * env::storage_byte_cost()), 
            len)
        );
        /*
            Ensure the attached deposit can cover: 
        */ 
        require!(current_user_balance >= required_deposit, "Not enough deposit");
        // Decrement the user's balance by the required deposit and insert back into the map
        current_user_balance -= required_deposit;
        self.user_balances.insert(&funder_id, &current_user_balance);
        env::log_str(&format!("New user balance {}", yocto_to_near(current_user_balance)));

        // Increment our fees earned
        self.fees_collected += self.drop_fee + self.key_fee * len;
        env::log_str(&format!("Fees collected {}", yocto_to_near(self.drop_fee + self.key_fee * len)));

        let current_account_id = env::current_account_id();
        
        /*
            Only add the access keys if it's not a FT drop. If it is,
            keys will be added in the FT resolver
        */
        if ft_data.is_none() {
            // Create a new promise batch to create all the access keys
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
        } else {
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
                Self::ext(current_account_id)
                    // Resolve the promise with the min GAS. All unspent GAS will be added to this call.
                    .with_static_gas(MIN_GAS_FOR_RESOLVE_STORAGE_CHECK)
                    .resolve_storage_check(
                        public_keys,
                        drop_id,
                        required_deposit
                    )
            );
        }

        drop_id
    }

    /*
        Allows users to add to an existing drop.
        Only the funder can call this method
    */
    #[payable]
    pub fn add_to_drop(
        &mut self, 
        public_keys: Vec<PublicKey>, 
        drop_id: DropId
    ) -> DropId {
        let mut drop = self.drop_for_id.get(&drop_id).expect("no drop found for ID");
        let funder = &drop.funder_id;

        require!(funder == &env::predecessor_account_id(), "only funder can add to drops");

        /*
            Add data to storage
        */
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();
        // get the existing key set and add new PKs
        let mut exiting_key_set = drop.pks;
        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in public_keys.clone() {
            exiting_key_set.insert(&pk);
            require!(self.drop_id_for_pk.insert(&pk, &drop_id).is_none(), "Keys cannot belong to another drop");
        }

        // Set the drop's PKs to the newly populated set
        drop.pks = exiting_key_set;

        // Add the drop back in for the drop ID 
        self.drop_for_id.insert(
            &drop_id, 
            &drop
        );
        
        // Get the current balance of the funder. 
        let mut current_user_balance = self.user_balances.get(&funder).expect("No user balance found");
        env::log_str(&format!("Cur user balance {}", yocto_to_near(current_user_balance)));
        
        // Get optional costs if the drop is not simple
        let fc_cost = drop.fc_data.as_ref().map(|data| data.deposit.0).unwrap_or(0);
        let ft_cost = drop.ft_data.as_ref().map(|data| data.ft_storage.unwrap().0).unwrap_or(0);
        let nft_cost = drop.nft_data.as_ref().map(|data| data.storage_for_longest * env::storage_byte_cost()).unwrap_or(0);
        
        let len = public_keys.len() as u128;
        
        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_required_storage = Balance::from(final_storage - initial_storage) * env::storage_byte_cost();
        env::log_str(&format!("Total required storage Yocto {}", total_required_storage));

        // Required deposit is the existing storage per key + key fee * length of public keys (plus all other basic stuff)
        let required_deposit = total_required_storage + (self.key_fee + ACCESS_KEY_ALLOWANCE + ACCESS_KEY_STORAGE + drop.balance.0 + fc_cost + ft_cost + nft_cost) * len;
        env::log_str(&format!(
            "Current User Balance: {}, 
            Required Deposit: {}, 
            Total required storage: {}, 
            Key fee: {}, 
            ACCESS_KEY_ALLOWANCE: {},
            ACCESS_KEY_STORAGE: {}, 
            Balance per key: {}, 
            function call costs: {}, 
            FT costs: {}, 
            NFT costs: {}, 
            length: {}", 
            yocto_to_near(current_user_balance), 
            yocto_to_near(required_deposit),
            yocto_to_near(total_required_storage),
            yocto_to_near(self.key_fee), 
            yocto_to_near(ACCESS_KEY_ALLOWANCE), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(drop.balance.0), 
            yocto_to_near(fc_cost), 
            yocto_to_near(ft_cost), 
            yocto_to_near(nft_cost), 
            len)
        );
        /*
            Ensure the attached deposit can cover: 
        */ 
        require!(current_user_balance >= required_deposit, "Not enough deposit");
        // Decrement the user's balance by the required deposit and insert back into the map
        current_user_balance -= required_deposit;
        self.user_balances.insert(&funder, &current_user_balance);
        env::log_str(&format!("New user balance {}", yocto_to_near(current_user_balance)));

        // Increment our fees earned
        self.fees_collected += self.key_fee * len;
        env::log_str(&format!("Fees collected {}", yocto_to_near(self.key_fee * len)));
        
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

        drop_id
    }
}