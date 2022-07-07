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
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct KeyUsage {
    // How many usages this key has. Once 0 is reached, the key is deleted
    pub num_uses: u64,

    // When was the last time the key was used
    pub last_used: u64,
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DropConfig {
    // How many claims can each key have
    pub max_claims_per_key: u64,
    
    // Minimum block timestamp that keys can be used. If None, keys can be used immediately
    // Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub start_timestamp: Option<u64>,

    // How often can a key be used 
    pub usage_interval: Option<u64>,

    // If regular claim is called and no account is created, should the balance be refunded to the funder
    pub refund_if_claim: Option<bool>,

    // Can the access key only call the claim method? Default to both method callable
    pub only_call_claim: Option<bool>
}

/// Keep track of specific data related to an access key. This allows us to optionally refund funders later. 
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Drop {
    // Funder of this specific drop
    pub funder_id: AccountId,
    // Set of public keys associated with this drop mapped to their usages
    pub pks: UnorderedMap<PublicKey, Option<KeyUsage>>,

    // Balance for all keys of this drop. Can be 0 if specified.
    pub balance: U128,

    // Every drop must have a type
    pub drop_type: DropType,

    // The drop as a whole can have a config as well
    pub drop_config: DropConfig,

    // How many claims
    pub num_claims_registered: u64,
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
        ft_data: Option<FTDataConfig>,
        nft_data: Option<NFTDataConfig>,
        fc_data: Option<FCData>,
        drop_config: DropConfig
    ) -> DropId {
        // Ensure the user has only specified one type of callback data
        let num_cbs_specified = ft_data.is_some() as u8 + nft_data.is_some() as u8 + fc_data.is_some() as u8;        
        env::log_str(&format!("Num CBs {}", num_cbs_specified));
        require!(num_cbs_specified <= 1, "You cannot specify more than one callback data");

        // Warn if the balance for each drop is less than the minimum
        if balance.0 < NEW_ACCOUNT_BASE {
            env::log_str(&format!("Warning: Balance is less than absolute minimum for creating an account: {}", NEW_ACCOUNT_BASE));
        }

        // Funder is the predecessor
        let funder_id = env::predecessor_account_id();
        let len = public_keys.len() as u128;
        let drop_id = self.nonce;

        // Get the current balance of the funder. 
        let mut current_user_balance = self.user_balances.get(&funder_id).expect("No user balance found");
        env::log_str(&format!("Cur User balance {}", yocto_to_near(current_user_balance)));
        
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();
        let mut key_map: UnorderedMap<PublicKey, Option<KeyUsage>> = UnorderedMap::new(StorageKey::PksForDrop {
            // We get a new unique prefix for the collection
            account_id_hash: hash_account_id(&format!("{}{}", self.nonce, funder_id)),
        });

        // Get the number of claims per key to dictate what key usage data we should put in the map
        let num_claims_per_key = drop_config.max_claims_per_key;
        require!(num_claims_per_key > 0, "cannot have less than 1 claim per key");
        // Create the key usage data structure based on whether the number of claims is more than 1
        let key_usage = if num_claims_per_key > 1 {
            Some(KeyUsage {
                num_uses: num_claims_per_key,
                last_used: 0 // Set to 0 since this will make the key always claimable.
            })
        } else {
            None
        };

        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in &public_keys {
            key_map.insert(pk, &key_usage);
            require!(self.drop_id_for_pk.insert(pk, &drop_id).is_none(), "Keys cannot belong to another drop");
        }

        // Add this drop ID to the funder's set of drops
        self.internal_add_drop_to_funder(&env::predecessor_account_id(), &drop_id);

        // Create drop object 
        let mut drop = Drop { 
            funder_id: env::predecessor_account_id(), 
            balance, 
            pks: key_map,
            drop_type: DropType::Simple, // Default to simple but will overwrite if not
            drop_config: drop_config.clone(),
            num_claims_registered: num_claims_per_key * len as u64
        };

        // Default the gas to attach to be the gas from the wallet. This will be used to calculate allowances.
        let mut gas_to_attach = ATTACHED_GAS_FROM_WALLET;

        // For NFT drops, measure the storage for adding the longest token ID
        let mut storage_per_longest = 0;
        // If NFT data was provided, we need to build the set of token IDs and cast the config to actual NFT data
        if let Some(data) = nft_data {
            let NFTDataConfig{nft_sender, nft_contract, longest_token_id} = data;

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
                token_ids,
            };

            // The number of claims is 0 until NFTs are sent to the contract
            drop.num_claims_registered = 0;
            drop.drop_type = DropType::NFT(actual_nft_data);
            
            // Add the drop with the empty token IDs
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
            
            // Measure how much storage it costs to insert the 1 longest token ID
            let initial_nft_storage_one = env::storage_usage();
            // Now that the drop has been added, insert the longest token ID and measure storage
            if let DropType::NFT(data) = &mut drop.drop_type {
                data.token_ids.insert(&longest_token_id);
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
            if let DropType::NFT(data) = &mut drop.drop_type {
                data.token_ids.clear();
                data.storage_for_longest = storage_per_longest;
            }

            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
        } else if let Some(data) = ft_data.clone() {
            // If FT Data was provided, we need to cast the FT Config to actual FT data and insert into the drop type
            let FTDataConfig{ft_sender, ft_contract, ft_balance} = data;

            // Create the NFT data
            let actual_ft_data = FTData {
                ft_contract,
                ft_sender,
                ft_balance,
                ft_storage: U128(u128::MAX),
            };

            // The number of claims is 0 until FTs are sent to the contract
            drop.num_claims_registered = 0;
            drop.drop_type = DropType::FT(actual_ft_data);
            
            // Add the drop with the empty token IDs
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
        } else if let Some(data) = fc_data.clone() {
            // If FC Data was provided, we need to set the drop type to be FC
            require!(data.gas_to_attach.unwrap_or(Gas(0)) <= ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE, &format!("cannot attach more than {:?} GAS.", ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE));
            // If GAS is specified, set the GAS to attach for allowance calculations
            if let Some(gas) = data.gas_to_attach {
                gas_to_attach = gas + GAS_OFFSET_IF_FC_EXECUTE;
            }
            
            drop.drop_type = DropType::FC(data);
            
            
            // Add the drop with the empty token IDs
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
        } else {
            // In simple case, we just insert the drop with whatever it was initialized with.
            self.drop_for_id.insert(
                &drop_id, 
                &drop
            );
        }

        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_required_storage = (Balance::from(final_storage - initial_storage) + storage_per_longest) * env::storage_byte_cost();
        env::log_str(&format!("Total required storage Yocto {}", total_required_storage));

        // Increment the drop ID nonce
        self.nonce += 1;

        // Dynamically calculate the access key allowance based on the base + number of claims per key * GAS to attach
        let burnt_gas = gas_to_attach.0 as u128 * GAS_PRICE;
        let access_key_allowance = BASE_ACCESS_KEY_ALLOWANCE + (num_claims_per_key - 1) as u128 * burnt_gas;

        let required_deposit = self.drop_fee + total_required_storage + (self.key_fee + access_key_allowance + (ACCESS_KEY_STORAGE + balance.0 + if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0} + storage_per_longest * env::storage_byte_cost()) * num_claims_per_key as u128) * len;
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
            Num claims per key: {}
            length: {}
            GAS to attach: {}", 
            yocto_to_near(current_user_balance), 
            yocto_to_near(required_deposit),
            yocto_to_near(self.drop_fee),
            yocto_to_near(total_required_storage), 
            yocto_to_near(self.key_fee),
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(access_key_allowance), 
            yocto_to_near(balance.0), 
            yocto_to_near(if fc_data.is_some() {fc_data.clone().unwrap().deposit.0} else {0}), 
            yocto_to_near(storage_per_longest * env::storage_byte_cost()), 
            num_claims_per_key,
            len,
            yocto_to_near(burnt_gas)
        ));
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
            // Decide what methods the access keys can call
            let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
            if drop_config.only_call_claim.is_some() {
                access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
            }

            // Create a new promise batch to create all the access keys
            let promise = env::promise_batch_create(&current_account_id);
            
            // Loop through each public key and create the access keys
            for pk in public_keys.clone() {
                // Must assert in the loop so no access keys are made?
                env::promise_batch_action_add_key_with_function_call(
                    promise, 
                    &pk, 
                    0, 
                    access_key_allowance, 
                    &current_account_id, 
                    access_key_method_names
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
        let drop_config = &drop.drop_config;
        let funder = &drop.funder_id;

        require!(funder == &env::predecessor_account_id(), "only funder can add to drops");

        let len = public_keys.len() as u128;

        /*
            Add data to storage
        */
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();
        
        // Get the number of claims per key
        let num_claims_per_key = drop_config.max_claims_per_key;
        // Create the key usage data structure based on whether the number of claims is more than 1
        let key_usage = if num_claims_per_key > 1 {
            Some(KeyUsage {
                num_uses: num_claims_per_key,
                last_used: 0 // Set to 0 since this will make the key always claimable.
            })
        } else {
            None
        };

        // get the existing key set and add new PKs
        let mut exiting_key_map = drop.pks;
        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in public_keys.clone() {
            exiting_key_map.insert(&pk, &key_usage);
            require!(self.drop_id_for_pk.insert(&pk, &drop_id).is_none(), "Keys cannot belong to another drop");
        }

        // Set the drop's PKs to the newly populated set
        drop.pks = exiting_key_map;

        // Default the gas to attach to be the gas from the wallet. This will be used to calculate allowances.
        let mut gas_to_attach = ATTACHED_GAS_FROM_WALLET;
        // Increment the claims registered if drop is FC or Simple
        match &drop.drop_type {
            DropType::FC(data) => {
                drop.num_claims_registered += num_claims_per_key * len as u64;
                
                // If GAS is specified, set the GAS to attach for allowance calculations
                if let Some(gas) = data.gas_to_attach {
                    gas_to_attach = gas + GAS_OFFSET_IF_FC_EXECUTE;
                }
            },
            DropType::Simple => {
                drop.num_claims_registered += num_claims_per_key * len as u64;
            },
            _ => {}
        };

        // Add the drop back in for the drop ID 
        self.drop_for_id.insert(
            &drop_id, 
            &drop
        );
        
        // Get the current balance of the funder. 
        let mut current_user_balance = self.user_balances.get(&funder).expect("No user balance found");
        env::log_str(&format!("Cur user balance {}", yocto_to_near(current_user_balance)));
        
        // Get optional costs for the drop
        let optional_costs = match drop.drop_type {
            DropType::FC(data) => {
                data.deposit.0
            },
            DropType::NFT(data) => {
                data.storage_for_longest * env::storage_byte_cost()
            },
            DropType::FT(data) => {
                data.ft_storage.0
            },
            _ => {0}
        };
        
        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_required_storage = Balance::from(final_storage - initial_storage) * env::storage_byte_cost();
        env::log_str(&format!("Total required storage Yocto {}", total_required_storage));

        // Dynamically calculate the access key allowance based on the base + number of claims per key * GAS to attach
        let burnt_gas = gas_to_attach.0 as u128 * GAS_PRICE;
        let access_key_allowance = BASE_ACCESS_KEY_ALLOWANCE + (num_claims_per_key - 1) as u128 * burnt_gas;

        // Required deposit is the existing storage per key + key fee * length of public keys (plus all other basic stuff)
        let required_deposit = total_required_storage + (self.key_fee + access_key_allowance + (ACCESS_KEY_STORAGE + drop.balance.0 + optional_costs) * num_claims_per_key as u128) * len;
        env::log_str(&format!(
            "Current User Balance: {}, 
            Required Deposit: {}, 
            Total required storage: {}, 
            Key fee: {}, 
            ACCESS_KEY_ALLOWANCE: {},
            ACCESS_KEY_STORAGE: {}, 
            Balance per key: {}, 
            optional costs: {}, 
            Number of claims per key: {},
            length: {}
            GAS to attach: {}", 
            yocto_to_near(current_user_balance), 
            yocto_to_near(required_deposit),
            yocto_to_near(total_required_storage),
            yocto_to_near(self.key_fee), 
            yocto_to_near(access_key_allowance), 
            yocto_to_near(ACCESS_KEY_STORAGE), 
            yocto_to_near(drop.balance.0), 
            yocto_to_near(optional_costs), 
            num_claims_per_key,
            len,
            yocto_to_near(burnt_gas)
        ));
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
            // Decide what methods the access keys can call
            let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
            if drop_config.only_call_claim.is_some() {
                access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
            }
            
            // Must assert in the loop so no access keys are made?
            env::promise_batch_action_add_key_with_function_call(
                promise, 
                &pk, 
                0, 
                access_key_allowance, 
                &current_account_id, 
                access_key_method_names
            );
        }

        env::promise_return(promise);

        drop_id
    }
}