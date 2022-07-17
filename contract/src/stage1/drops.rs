use crate::*;
use near_sdk::{require, Balance};

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

    // How much allowance does the key have left. When the key is deleted, this is refunded to the funder's balance.
    pub allowance: u128,
}

/// Keep track of different configuration options for each key in a drop
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct DropConfig {
    // How many claims can each key have. If None, default to 1.
    pub max_claims_per_key: Option<u64>,

    // Minimum block timestamp that keys can be used. If None, keys can be used immediately
    // Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub start_timestamp: Option<u64>,

    // How often can a key be used
    pub usage_interval: Option<u64>,

    // If regular claim is called and no account is created, should the balance be refunded to the funder
    pub refund_if_claim: Option<bool>,

    // Can the access key only call the claim method? Default to both method callable
    pub only_call_claim: Option<bool>,
}

// Drop Metadata should be a string which can be JSON or anything the users want.
pub type DropMetadata = String;

/// Keep track of specific data related to an access key. This allows us to optionally refund funders later.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Drop {
    // Funder of this specific drop
    pub funder_id: AccountId,
    // Set of public keys associated with this drop mapped to their usages
    pub pks: UnorderedMap<PublicKey, KeyUsage>,

    // Balance for all keys of this drop. Can be 0 if specified.
    pub balance: U128,

    // How many claims
    pub num_claims_registered: u64,

    // Ensure this drop can only be used when the function has the required gas to attach
    pub required_gas_attached: Gas,

    // Every drop must have a type
    pub drop_type: DropType,

    // The drop as a whole can have a config as well
    pub drop_config: Option<DropConfig>,

    // Metadata for the drop
    pub drop_metadata: Option<DropMetadata>,
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
        drop_config: Option<DropConfig>,
        drop_metadata: Option<DropMetadata>,
        ft_data: Option<FTDataConfig>,
        nft_data: Option<NFTDataConfig>,
        fc_data: Option<FCData>,
    ) -> DropId {
        // Ensure the user has only specified one type of callback data
        let num_cbs_specified =
            ft_data.is_some() as u8 + nft_data.is_some() as u8 + fc_data.is_some() as u8;
        require!(
            num_cbs_specified <= 1,
            "You cannot specify more than one callback data"
        );

        // Warn if the balance for each drop is less than the minimum
        if balance.0 < NEW_ACCOUNT_BASE {
            near_sdk::log!(
                "Warning: Balance is less than absolute minimum for creating an account: {}",
                NEW_ACCOUNT_BASE
            );
        }

        // Funder is the predecessor
        let funder_id = env::predecessor_account_id();
        let len = public_keys.len() as u128;
        let drop_id = self.nonce;
        // Get the number of claims per key to dictate what key usage data we should put in the map
        let num_claims_per_key = drop_config
            .clone()
            .and_then(|c| c.max_claims_per_key)
            .unwrap_or(1);
        require!(
            num_claims_per_key > 0,
            "cannot have less than 1 claim per key"
        );

        // Get the current balance of the funder.
        let mut current_user_balance = self
            .user_balances
            .get(&funder_id)
            .expect("No user balance found");
        near_sdk::log!("Cur User balance {}", yocto_to_near(current_user_balance));

        // Pessimistically measure storage
        let initial_storage = env::storage_usage();
        let mut key_map: UnorderedMap<PublicKey, KeyUsage> =
            UnorderedMap::new(StorageKey::PksForDrop {
                // We get a new unique prefix for the collection
                account_id_hash: hash_account_id(&format!("{}{}", self.nonce, funder_id)),
            });

        // Decide what methods the access keys can call
        let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
        if drop_config
            .clone()
            .and_then(|c| c.only_call_claim)
            .unwrap_or(false)
        {
            access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
        }

        // Default the gas to attach to be the gas from the wallet. This will be used to calculate allowances.
        let mut gas_to_attach = ATTACHED_GAS_FROM_WALLET;
        // Depending on the FC Data, set the Gas to attach and the access key method names
        if let Some(gas) = fc_data
            .clone()
            .and_then(|d| d.config.and_then(|c| c.gas_if_claim_only))
        {
            require!(
                balance.0 == 0,
                "cannot specify gas to attach and have a balance in the linkdrop"
            );
            require!(
                gas <= ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE,
                &format!(
                    "cannot attach more than {:?} GAS.",
                    ATTACHED_GAS_FROM_WALLET - GAS_OFFSET_IF_FC_EXECUTE
                )
            );
            gas_to_attach = gas + GAS_OFFSET_IF_FC_EXECUTE;
            access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
        }

        // Calculate the base allowance to attach
        let calculated_base_allowance = self.calculate_base_allowance(gas_to_attach);
        // The actual allowance is the base * number of claims per key since each claim can potentially use the max pessimistic GAS.
        let actual_allowance = calculated_base_allowance * num_claims_per_key as u128;

        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in &public_keys {
            key_map.insert(
                pk,
                &KeyUsage {
                    num_uses: num_claims_per_key,
                    last_used: 0, // Set to 0 since this will make the key always claimable.
                    allowance: actual_allowance,
                },
            );
            require!(
                self.drop_id_for_pk.insert(pk, &drop_id).is_none(),
                "Keys cannot belong to another drop"
            );
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
            num_claims_registered: num_claims_per_key * len as u64,
            required_gas_attached: gas_to_attach,
            drop_metadata,
        };

        // For NFT drops, measure the storage for adding the longest token ID
        let mut storage_per_longest = 0;
        // Keep track of the total deposit required for the FC data (depending on None and Some cases)
        let mut deposit_required_for_fc_deposits = 0;
        // Keep track of the number of none FCs so we don't charge the user
        let mut num_none_fcs = 0;
        // If NFT data was provided, we need to build the set of token IDs and cast the config to actual NFT data
        if let Some(data) = nft_data {
            let NFTDataConfig {
                nft_sender,
                nft_contract,
                longest_token_id,
            } = data;

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
            self.drop_for_id.insert(&drop_id, &drop);

            // Measure how much storage it costs to insert the 1 longest token ID
            let initial_nft_storage_one = env::storage_usage();
            // Now that the drop has been added, insert the longest token ID and measure storage
            if let DropType::NFT(data) = &mut drop.drop_type {
                data.token_ids.insert(&longest_token_id);
            }

            // Add drop with the longest possible token ID and max storage
            self.drop_for_id.insert(&drop_id, &drop);
            let final_nft_storage_one = env::storage_usage();
            near_sdk::log!(
                "i1: {} f1: {}",
                initial_nft_storage_one,
                final_nft_storage_one
            );

            // Measure the storage per single longest token ID
            storage_per_longest = Balance::from(final_nft_storage_one - initial_nft_storage_one);
            near_sdk::log!(
                "TOKENS BEFORE {:?}",
                self.get_token_ids_for_drop(self.nonce, None, None)
            );

            // Clear the token IDs so it's an empty set and put the storage in the drop's nft data
            if let DropType::NFT(data) = &mut drop.drop_type {
                data.token_ids.clear();
                data.storage_for_longest = storage_per_longest;
            }

            self.drop_for_id.insert(&drop_id, &drop);
        } else if let Some(data) = ft_data.clone() {
            // If FT Data was provided, we need to cast the FT Config to actual FT data and insert into the drop type
            let FTDataConfig {
                ft_sender,
                ft_contract,
                ft_balance,
            } = data;

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
            self.drop_for_id.insert(&drop_id, &drop);
        } else if let Some(data) = fc_data.clone() {
            drop.drop_type = DropType::FC(data.clone());

            // Ensure proper method data is passed in
            let num_fcs = data.clone().method_data.len() as u64;
            // If there's 1 claim, there should be 1 method defined
            if num_claims_per_key == 1 {
                require!(
                    num_fcs == 1,
                    "Cannot have more FCs than the number of claims per key"
                );
            // If there's more than 1 method defined, the number of methods should equal the number of claims per key
            } else if num_fcs > 1 {
                require!(
                    num_fcs == num_claims_per_key,
                    "Number of FCs must match number of claims per key if more than 1 is specified"
                );
            }

            // If there's one FC specified and more than 1 claim per key, that FC is to be used
            // For all the claims. In this case, we need to tally all the deposits for each claim.
            if num_claims_per_key > 1 && num_fcs == 1 {
                let deposit = data
                    .method_data
                    .iter()
                    .next()
                    .unwrap()
                    .clone()
                    .expect("cannot have a single none function call")
                    .deposit
                    .0;
                deposit_required_for_fc_deposits = num_claims_per_key as u128 * deposit;

            // In the case where either there's 1 claim per key or the number of FCs is not 1,
            // We can simply loop through and manually get this data
            } else {
                for method in data.method_data {
                    num_none_fcs += method.is_some() as u64;
                    deposit_required_for_fc_deposits += method.map(|m| m.deposit.0).unwrap_or(0);
                }
            }

            // Add the drop with the empty token IDs
            self.drop_for_id.insert(&drop_id, &drop);
        } else {
            require!(balance.0 > 0, "Cannot have a simple drop with zero balance");
            // In simple case, we just insert the drop with whatever it was initialized with.
            self.drop_for_id.insert(&drop_id, &drop);
        }

        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_required_storage = (Balance::from(final_storage - initial_storage)
            + storage_per_longest)
            * env::storage_byte_cost();
        near_sdk::log!("Total required storage Yocto {}", total_required_storage);

        // Increment the drop ID nonce
        self.nonce += 1;

        /*
            Required deposit consists of:
            - Fees
            - TOTAL Storage
            - Total access key allowance for EACH key
            - Access key storage for EACH key
            - Balance for each key * (number of claims - claims with None for FC Data)

            Optional:
            - FC deposit for each key * num Some(data) claims
            - storage for longest token ID for each key
            - FT storage registration cost for each key * claims (calculated in resolve storage calculation function)
        */
        let required_deposit = self.drop_fee
            + total_required_storage
            + (self.key_fee
                + actual_allowance
                + ACCESS_KEY_STORAGE
                + balance.0 * (num_claims_per_key - num_none_fcs) as u128
                + storage_per_longest * env::storage_byte_cost()
                + deposit_required_for_fc_deposits)
                * len;
        near_sdk::log!(
            "Current balance: {}, 
            Required Deposit: {}, 
            Drop Fee: {}, 
            Total Required Storage: {}, 
            Key Fee: {}, 
            ACCESS_KEY_ALLOWANCE: {}, 
            ACCESS_KEY_STORAGE: {},
            Linkdrop Balance: {}, 
            Storage for longest token ID (if applicable): {},
            total function call deposits (if applicable): {},
            Num claims per key: {}
            Num none FCs: {},
            length: {}
            GAS to attach: {}",
            yocto_to_near(current_user_balance),
            yocto_to_near(required_deposit),
            yocto_to_near(self.drop_fee),
            yocto_to_near(total_required_storage),
            yocto_to_near(self.key_fee),
            yocto_to_near(actual_allowance),
            yocto_to_near(ACCESS_KEY_STORAGE),
            yocto_to_near(balance.0),
            yocto_to_near(storage_per_longest * env::storage_byte_cost()),
            yocto_to_near(deposit_required_for_fc_deposits),
            num_claims_per_key,
            num_none_fcs,
            len,
            gas_to_attach.0
        );

        /*
            Ensure the attached deposit can cover:
        */
        require!(
            current_user_balance >= required_deposit,
            "Not enough deposit"
        );
        // Decrement the user's balance by the required deposit and insert back into the map
        current_user_balance -= required_deposit;
        self.user_balances.insert(&funder_id, &current_user_balance);
        near_sdk::log!("New user balance {}", yocto_to_near(current_user_balance));

        // Increment our fees earned
        self.fees_collected += self.drop_fee + self.key_fee * len;
        near_sdk::log!(
            "Fees collected {}",
            yocto_to_near(self.drop_fee + self.key_fee * len)
        );

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
                    actual_allowance,
                    &current_account_id,
                    access_key_method_names,
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
                        .resolve_storage_check(public_keys, drop_id, required_deposit),
                );
        }

        drop_id
    }

    /*
        Allows users to add to an existing drop.
        Only the funder can call this method
    */
    #[payable]
    pub fn add_to_drop(&mut self, public_keys: Vec<PublicKey>, drop_id: DropId) -> DropId {
        let mut drop = self
            .drop_for_id
            .get(&drop_id)
            .expect("no drop found for ID");
        let drop_config = &drop.drop_config;
        let funder = &drop.funder_id;

        require!(
            funder == &env::predecessor_account_id(),
            "only funder can add to drops"
        );

        let len = public_keys.len() as u128;

        /*
            Add data to storage
        */
        // Pessimistically measure storage
        let initial_storage = env::storage_usage();

        // Get the number of claims per key
        let num_claims_per_key = drop_config
            .clone()
            .and_then(|c| c.max_claims_per_key)
            .unwrap_or(1);

        // get the existing key set and add new PKs
        let mut exiting_key_map = drop.pks;

        // Calculate the base allowance to attach
        let calculated_base_allowance = self.calculate_base_allowance(drop.required_gas_attached);
        // The actual allowance is the base * number of claims per key since each claim can potentially use the max pessimistic GAS.
        let actual_allowance = calculated_base_allowance * num_claims_per_key as u128;
        // Loop through and add each drop ID to the public keys. Also populate the key set.
        for pk in public_keys.clone() {
            exiting_key_map.insert(
                &pk,
                &KeyUsage {
                    num_uses: num_claims_per_key,
                    last_used: 0, // Set to 0 since this will make the key always claimable.
                    allowance: actual_allowance,
                },
            );
            require!(
                self.drop_id_for_pk.insert(&pk, &drop_id).is_none(),
                "Keys cannot belong to another drop"
            );
        }

        // Set the drop's PKs to the newly populated set
        drop.pks = exiting_key_map;

        // Decide what methods the access keys can call
        let mut access_key_method_names = ACCESS_KEY_BOTH_METHOD_NAMES;
        if drop_config
            .clone()
            .and_then(|c| c.only_call_claim)
            .unwrap_or(false)
        {
            access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
        }

        // Increment the claims registered if drop is FC or Simple
        match &drop.drop_type {
            DropType::FC(data) => {
                drop.num_claims_registered += num_claims_per_key * len as u64;

                // If GAS is specified, set the GAS to attach for allowance calculations
                if let Some(_) = data.config.clone().and_then(|c| c.gas_if_claim_only) {
                    access_key_method_names = ACCESS_KEY_CLAIM_METHOD_NAME;
                }
            }
            DropType::Simple => {
                drop.num_claims_registered += num_claims_per_key * len as u64;
            }
            _ => {}
        };

        // Add the drop back in for the drop ID
        self.drop_for_id.insert(&drop_id, &drop);

        // Get the current balance of the funder.
        let mut current_user_balance = self
            .user_balances
            .get(&funder)
            .expect("No user balance found");
        near_sdk::log!("Cur user balance {}", yocto_to_near(current_user_balance));

        // Get the required deposit for all the FCs
        let mut deposit_required_for_fc_deposits = 0;
        // Get the number of none FCs in FCData (if there are any)
        let mut num_none_fcs = 0;
        if let DropType::FC(data) = &drop.drop_type {
            let num_fcs = data.method_data.len() as u64;

            // If there's one FC specified and more than 1 claim per key, that FC is to be used
            // For all the claims. In this case, we need to tally all the deposits for each claim.
            if num_claims_per_key > 1 && num_fcs == 1 {
                let deposit = data
                    .method_data
                    .iter()
                    .next()
                    .unwrap()
                    .clone()
                    .expect("cannot have a single none function call")
                    .deposit
                    .0;
                deposit_required_for_fc_deposits = num_claims_per_key as u128 * deposit;

            // In the case where either there's 1 claim per key or the number of FCs is not 1,
            // We can simply loop through and manually get this data
            } else {
                for method in data.method_data.clone() {
                    num_none_fcs += method.is_some() as u64;
                    deposit_required_for_fc_deposits += method.map(|m| m.deposit.0).unwrap_or(0);
                }
            }
        }

        // Get optional costs
        let mut nft_optional_costs_per_key = 0;
        let mut ft_optional_costs_per_claim = 0;
        match drop.drop_type {
            DropType::NFT(data) => {
                nft_optional_costs_per_key = data.storage_for_longest * env::storage_byte_cost()
            }
            DropType::FT(data) => ft_optional_costs_per_claim = data.ft_storage.0,
            _ => {}
        };

        // Calculate the storage being used for the entire drop
        let final_storage = env::storage_usage();
        let total_required_storage =
            Balance::from(final_storage - initial_storage) * env::storage_byte_cost();
        near_sdk::log!("Total required storage Yocto {}", total_required_storage);

        /*
            Required deposit consists of:
            - Fees
            - TOTAL Storage
            - Total access key allowance for EACH key
            - Access key storage for EACH key
            - Balance for each key * (number of claims - claims with None for FC Data)

            Optional:
            - FC deposit for each key * num Some(data) claims
            - storage for longest token ID for each key
            - FT storage registration cost for each key * claims (calculated in resolve storage calculation function)
        */
        let required_deposit = total_required_storage
            + (self.key_fee
                + actual_allowance
                + ACCESS_KEY_STORAGE
                + drop.balance.0 * (num_claims_per_key - num_none_fcs) as u128
                + nft_optional_costs_per_key
                + deposit_required_for_fc_deposits
                + ft_optional_costs_per_claim * num_claims_per_key as u128)
                * len;

        near_sdk::log!(
            "Current balance: {}, 
            Required Deposit: {},  
            Total Required Storage: {}, 
            Key Fee: {}, 
            ACCESS_KEY_ALLOWANCE: {}, 
            ACCESS_KEY_STORAGE: {},
            Linkdrop Balance: {}, 
            NFT Optional costs per key: {},
            total function call deposits per key: {},
            FT Optional costs per claim: {},
            Num claims per key: {}
            Num none FCs: {},
            length: {}",
            yocto_to_near(current_user_balance),
            yocto_to_near(required_deposit),
            yocto_to_near(total_required_storage),
            yocto_to_near(self.key_fee),
            yocto_to_near(actual_allowance),
            yocto_to_near(ACCESS_KEY_STORAGE),
            yocto_to_near(drop.balance.0),
            yocto_to_near(nft_optional_costs_per_key),
            yocto_to_near(deposit_required_for_fc_deposits),
            yocto_to_near(ft_optional_costs_per_claim),
            num_claims_per_key,
            num_none_fcs,
            len,
        );
        /*
            Ensure the attached deposit can cover:
        */
        require!(
            current_user_balance >= required_deposit,
            "Not enough deposit"
        );
        // Decrement the user's balance by the required deposit and insert back into the map
        current_user_balance -= required_deposit;
        self.user_balances.insert(&funder, &current_user_balance);
        near_sdk::log!("New user balance {}", yocto_to_near(current_user_balance));

        // Increment our fees earned
        self.fees_collected += self.key_fee * len;
        near_sdk::log!("Fees collected {}", yocto_to_near(self.key_fee * len));

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
                actual_allowance,
                &current_account_id,
                access_key_method_names,
            );
        }

        env::promise_return(promise);

        drop_id
    }
}
