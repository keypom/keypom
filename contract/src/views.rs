use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum JsonDropType {
    Simple,
    NonFungibleToken(JsonNFTData),
    FungibleToken(FTData),
    FunctionCall(FCData),
}

/// Struct to return in views to query for drop info
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonDrop {
    // Drop ID for this drop
    pub drop_id: DropId,
    // owner of this specific drop
    pub owner_id: AccountId,

    // Balance for all keys of this drop. Can be 0 if specified.
    pub deposit_per_use: U128,

    // Every drop must have a type
    pub drop_type: JsonDropType,

    // The drop as a whole can have a config as well
    pub config: Option<DropConfig>,

    // Metadata for the drop
    pub metadata: Option<DropMetadata>,

    // How many claims
    pub registered_uses: u64,

    // Ensure this drop can only be used when the function has the required gas to attach
    pub required_gas: Gas,

    // Keep track of the next nonce to give out to a key
    pub next_key_id: u64,
}

/// Keep track of nft data
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonNFTData {
    pub sender_id: AccountId,
    pub contract_id: AccountId,
    pub longest_token_id: String,
    pub storage_for_longest: U128,
}

/// Struct to return in views to query for specific data related to an access key.
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonKeyInfo {
    // Drop ID for the specific drop
    pub drop_id: DropId,
    pub pk: PublicKey,
    pub key_info: KeyInfo,
}

#[near_bindgen]
impl Keypom {
    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let drop_id = self
            .drop_id_for_pk
            .get(&key)
            .expect("no drop ID found for key");
        let drop = self
            .drop_for_id
            .get(&drop_id)
            .expect("no drop found for drop ID");
        U128(drop.deposit_per_use)
    }

    /*
        CUSTOM
    */
    /// Query for the total supply of keys on the contract
    pub fn get_key_total_supply(&self) -> U128 {
        //return the length of the data_for_pk set
        U128(self.drop_id_for_pk.len() as u128)
    }

    /// Paginate through all active keys on the contract and return a vector of key info.
    pub fn get_keys(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<JsonKeyInfo> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        self.drop_id_for_pk
            .keys()
            //skip to the index we specified in the start variable
            .skip(start as usize)
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize)
            //we'll map the public key which are strings into Drops
            .map(|pk| self.get_key_information(pk.clone()))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    /// Returns the JsonKeyInfo corresponding to a specific key
    pub fn get_key_information(&self, key: PublicKey) -> JsonKeyInfo {
        let drop_id = self
            .drop_id_for_pk
            .get(&key)
            .expect("no drop ID found for key");
        let drop = self
            .drop_for_id
            .get(&drop_id)
            .expect("no drop found for drop ID");
        let key_info = drop.pks.get(&key).unwrap();

        JsonKeyInfo {
            key_info,
            drop_id,
            pk: key,
        }
    }

    /// Returns the JsonDrop corresponding to a drop ID. If a key is specified, it will return the drop info for that key.
    pub fn get_drop_information(
        &self,
        drop_id: Option<DropId>,
        key: Option<PublicKey>,
    ) -> JsonDrop {
        // If the user doesn't specify a drop ID or a key, panic.
        if drop_id.is_none() && key.is_none() {
            env::panic_str("must specify either a drop ID or a public key");
        }

        // Set the drop ID to be what was passed in. If they didn't pass in a drop ID, get it
        let mut drop_id = drop_id.unwrap_or(0);

        // If the user specifies a key, use that to get the drop ID.
        if let Some(key) = key {
            drop_id = self.drop_id_for_pk.get(&key).expect("no drop ID for PK");
        }

        let drop = self
            .drop_for_id
            .get(&drop_id)
            .expect("no drop found for drop ID");

        let drop_type: JsonDropType = match drop.drop_type {
            DropType::FunctionCall(data) => JsonDropType::FunctionCall(data),
            DropType::NonFungibleToken(data) => JsonDropType::NonFungibleToken(JsonNFTData {
                contract_id: data.contract_id,
                sender_id: data.sender_id,
                longest_token_id: data.longest_token_id,
                storage_for_longest: U128(data.storage_for_longest),
            }),
            DropType::FungibleToken(data) => JsonDropType::FungibleToken(data),
            _simple => JsonDropType::Simple,
        };

        JsonDrop {
            drop_id,
            owner_id: drop.owner_id,
            deposit_per_use: U128(drop.deposit_per_use),
            drop_type,
            config: drop.config,
            registered_uses: drop.registered_uses,
            required_gas: drop.required_gas,
            metadata: drop.metadata.get(),
            next_key_id: drop.next_key_id,
        }
    }

    /// Returns the total supply of active keys for a given drop
    pub fn get_key_supply_for_drop(&self, drop_id: DropId) -> u64 {
        // Get the drop object and return the length
        self.drop_for_id
            .get(&drop_id)
            .expect("no drop found")
            .pks
            .len()
    }

    /// Paginate through keys in a specific drop
    pub fn get_keys_for_drop(
        &self,
        drop_id: DropId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<JsonKeyInfo> {
        //where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
        let start = u128::from(from_index.unwrap_or(U128(0)));

        //iterate through each key using an iterator
        self.drop_for_id
            .get(&drop_id)
            .expect("No drop for given ID")
            .pks
            .keys()
            //skip to the index we specified in the start variable
            .skip(start as usize)
            //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
            .take(limit.unwrap_or(50) as usize)
            //we'll map the public key which are strings into Drops
            .map(|pk| self.get_key_information(pk.clone()))
            //since we turned the keys into an iterator, we need to turn it back into a vector to return
            .collect()
    }

    /// Returns the total supply of active drops for a given owner
    pub fn get_drop_supply_for_owner(&self, account_id: AccountId) -> u64 {
        //get the set of drops for the passed in owner
        let drops_for_owner = self.drop_ids_for_owner.get(&account_id);

        //if there is some set of drops, we'll return the length
        if let Some(drops_for_owner) = drops_for_owner {
            drops_for_owner.len()
        } else {
            //if there isn't a set of keys for the passed in account ID, we'll return 0
            0
        }
    }

    /// Return a vector of drop information for a owner
    pub fn get_drops_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<JsonDrop> {
        // Iterate through each drop ID and push JsonDrop to a vector
        let drop_ids = self.drop_ids_for_owner.get(&account_id);

        // If there are IDs, iterate and create the vector of JsonDrops otherwise return empty array.s
        if let Some(ids) = drop_ids {
            // Where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
            let start = u128::from(from_index.unwrap_or(U128(0)));

            ids.iter()
                // Skip to the index we specified in the start variable
                .skip(start as usize)
                // Take the first "limit" elements in the vector. If we didn't specify a limit, use 50
                .take(limit.unwrap_or(50) as usize)
                // Convert each ID into a JsonDrop
                .map(|id| self.get_drop_information(Some(id), None))
                // Collect all JsonDrops into a vector and return it
                .collect()
        } else {
            return vec![];
        }
    }

    /// Return the total supply of token IDs for a given drop
    pub fn get_nft_supply_for_drop(&self, drop_id: DropId) -> u64 {
        let drop = self.drop_for_id.get(&drop_id).expect("no drop found");
        if let DropType::NonFungibleToken(nft_data) = drop.drop_type {
            return nft_data.token_ids.len();
        } else {
            return 0;
        }
    }

    /// Paginate through token IDs in a drop
    pub fn get_nft_token_ids_for_drop(
        &self,
        drop_id: DropId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<String> {
        let drop = self.drop_for_id.get(&drop_id).expect("no drop found");
        if let DropType::NonFungibleToken(nft_data) = drop.drop_type {
            let token_ids = nft_data.token_ids;

            // Where to start pagination - if we have a from_index, we'll use that - otherwise start from 0 index
            let start = u128::from(from_index.unwrap_or(U128(0)));

            // Iterate through each token ID using an iterator
            token_ids
                .iter()
                //skip to the index we specified in the start variable
                .skip(start as usize)
                //take the first "limit" elements in the vector. If we didn't specify a limit, use 50
                .take(limit.unwrap_or(50) as usize)
                //since we turned the keys into an iterator, we need to turn it back into a vector to return
                .collect()
        } else {
            return vec![];
        }
    }

    /// Returns the current nonce on the contract
    pub fn get_next_drop_id(&self) -> u128 {
        self.next_drop_id
    }

    /// Returns how many fees the contract has collected
    pub fn get_fees_collected(&self) -> U128 {
        U128(self.fees_collected)
    }

    /// Returns the current GAS price stored on the contract
    pub fn get_gas_price(&self) -> U128 {
        U128(self.yocto_per_gas)
    }

    /// Returns the current linkdrop contract
    pub fn get_root_account(&self) -> String {
        self.root_account.to_string()
    }

    /// Returns the current fees associated with an account
    pub fn get_fees_per_user(&self, account_id: AccountId) -> Option<(U128, U128)> {
        // return fees per user as a U128 tuple
        self.fees_per_user
            .get(&account_id)
            .map(|fees| (U128(fees.0), U128(fees.1)))
    }
}
