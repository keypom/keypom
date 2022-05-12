use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseResult, PublicKey,
};

/* 
    minimum amount of storage required to cover:
    - storing access key on the contract
    - storing pub key and account data in the map
    Some of this can be refunded once the account is claimed.
*/ 
const STORAGE_ALLOWANCE: u128 = 10_000_000_000_000_000_000_000; // 0.01 N 
/* 
    allowance for the access key to cover GAS fees when the account is claimed. This amount is will not be "reserved" on the contract but must be 
    available when GAS is burnt using the access key. The burnt GAS will not be refunded but any unburnt GAS that remains can be refunded.

    If this is lower, wallet will throw the following error:
    Access Key {account_id}:{public_key} does not have enough balance 0.01 for transaction costing 0.018742491841859367297184
*/  
const ACCESS_KEY_ALLOWANCE: u128 = 20_000_000_000_000_000_000_000; // 0.02 N 
/* 
    minimum amount of NEAR that a new account (with longest possible name) must have when created 
    If this is less, it will throw a lack balance for state error (assuming you have the same account ID length)
*/ 
const NEW_ACCOUNT_BASE: u128 = 2_840_000_000_000_000_000_000; // 0.00284 N

const ON_CREATE_ACCOUNT_GAS: Gas = Gas(40_000_000_000_000); // 40 TGas
const ON_CALLBACK_GAS: Gas = Gas(20_000_000_000_000); // 20 TGas

// Defaulting burnt GAS to be 300 TGas (0.03 $NEAR)
const BURNT_GAS: u128 = 30_000_000_000_000_000_000_000;

/// Indicates there are no deposit for a callback for better readability.
const NO_DEPOSIT: u128 = 0;

/// Methods callable by the function call access key
const ACCESS_KEY_METHOD_NAMES: &str = "claim,create_account_and_claim";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Accounts,
}
/// Keep track of specific data related to an access key. This allows us to optionally refund funders later. 
#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct AccountData {
    pub funder_id: AccountId,
    pub balance: U128,
}

mod ext_traits;
use crate::ext_traits::*;

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct LinkDropProxy {
    pub linkdrop_contract: AccountId,
    pub accounts: LookupMap<PublicKey, AccountData>,
}

#[near_bindgen]
impl LinkDropProxy {
    /// Initialize proxy hub contract and pass in the desired deployed linkdrop contract (i.e testnet or near)
    #[init]
    pub fn new(linkdrop_contract: AccountId) -> Self {
        Self {
            linkdrop_contract,
            accounts: LookupMap::new(StorageKey::Accounts),
        }
    }

    /// Set the desired linkdrop contract to interact with
	pub fn set_contract(&mut self, linkdrop_contract: AccountId) {
		assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
		self.linkdrop_contract = linkdrop_contract;
	}

	/*
        user has created a keypair and passes in the public key and attaches some deposit.
        this will store the account data and allow that key to call claim and create_account_and_claim
        on this contract.

        The balance is the amount of $NEAR the sender wants the linkdrop to contain.
    */
    #[payable]
    pub fn send(&mut self, public_key: PublicKey, balance: U128) -> Promise {
        let attached_deposit = env::attached_deposit();

        /*
            ensure the user attached enough to cover:
            - storage allowance on the contract for access key and storing account data and pk
            - access key allowance 
            - and a balance for the account (which must be greater than new account base)
        */
        assert!(
            attached_deposit >= balance.0 + ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE,
            "Deposit must be large enough to cover desired balance, access key allowance, and contract storage"
        );

        assert!(
            balance.0 >= NEW_ACCOUNT_BASE,
            "cannot have a desired account balance less than the absolute minimum for creating an account"
        );

        
        let pk = public_key;
        assert!(self.accounts.insert(
                &pk,
                &AccountData{
                    funder_id: env::predecessor_account_id(),
                    balance: balance,
                },
            ).is_none(),
            "Account for PublicKey exists"
        );

        // If the user overpaid for the desired linkdrop balance, refund them.
        if attached_deposit > balance.0 + ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE {
            Promise::new(env::predecessor_account_id()).transfer(attached_deposit - balance.0 - ACCESS_KEY_ALLOWANCE - STORAGE_ALLOWANCE);
        }

        /*
            add the public key as an access key to the contract 
            which can only call claim and create_account_and_claim on this contract
        */
        Promise::new(env::current_account_id()).add_access_key(
            pk,
            ACCESS_KEY_ALLOWANCE,
            env::current_account_id(),
            ACCESS_KEY_METHOD_NAMES.to_string(),
        )

        //TODO: do assertions at the end and dynamically calculate required storage? Store this in the account data?
    }

    /*
        user has created a bunch of keypairs and passesd in the public keys and attached some deposit.
        this will store the account data and allow that keys to call claim and create_account_and_claim
        on this contract.

        The balance is the amount of $NEAR the sender wants each linkdrop to contain.
    */
    #[payable]
    pub fn send_multiple(&mut self, public_keys: Vec<PublicKey>, balance: U128) {
	    
        let attached_deposit = env::attached_deposit();
		let len = public_keys.len() as u128;
	    
        /*
            ensure the user attached enough to cover:
            - storage allowance on the contract for access key and storing account data and pk
            - access key allowance 
            - and a balance for the account (which must be greater than new account base)

            this must be true for every public key passed in.
        */
        assert!(
            attached_deposit >= (balance.0 + ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE) * len,
            "Deposit must be large enough to cover desired balance, access key allowance, and contract storage for all keys"
        );

        assert!(
            balance.0 >= NEW_ACCOUNT_BASE,
            "cannot have a desired account balance less than the absolute minimum for creating an account"
        );

		let current_account_id = env::current_account_id();
		let promise = env::promise_batch_create(&current_account_id);
        let funder_id = env::predecessor_account_id();
		
        // Loop through each public key and insert into the map and create the key
		for pk in public_keys {

			env::promise_batch_action_add_key_with_function_call(
				promise, 
				&pk, 
				0, 
				ACCESS_KEY_ALLOWANCE, 
				&current_account_id, 
				ACCESS_KEY_METHOD_NAMES
			);
			
            // Ensure none of the public keys exist already
			assert!(self.accounts.insert(
                    &pk,
                    &AccountData{
                        funder_id: funder_id.clone(),
                        balance,
                    },
                ).is_none(),
                "Account for PublicKey exists"
            );
		}

		env::promise_return(promise);
    }

	
    /// Internal method for deleting the used key and removing / returning account data.
	fn process_claim(&mut self) -> (PublicKey, U128, AccountId) {
        // Ensure only the current contract is calling the method using the access key
		assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );

        // Get the PK of the signer which should be the contract's function call access key
        let signer_pk = env::signer_account_pk();
        let AccountData {
            funder_id,
            balance,
        } = self.accounts
            .remove(&signer_pk)
            .expect("Missing public key");

        // Delete the key
		Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());

        // Return account data info
		(signer_pk, balance, funder_id)
	}

    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) -> Promise {
        // Delete the access key and remove / return account data.
        let (signer_pk, balance, funder_id) = self.process_claim();
		
        // Send the existing account ID the desired linkdrop balance.
        Promise::new(account_id).transfer(balance.0)
        .then(ext_self::on_claim(
			signer_pk,
            balance,
            funder_id,
			env::current_account_id(),
			NO_DEPOSIT,
			ON_CALLBACK_GAS,
		))
    }

    /// Create new account and and claim tokens to it.
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: AccountId,
        new_public_key: PublicKey,
    ) -> Promise {
        // Delete the access key and remove / return account data.
        let (signer_pk, balance, funder_id) = self.process_claim();

        // CCC to the linkdrop contract to create the account with the desired balance as the linkdrop amount
        ext_linkdrop::create_account(
            new_account_id,
            new_public_key,
            self.linkdrop_contract.clone(),
            balance.0,
            ON_CREATE_ACCOUNT_GAS,
        ).then(ext_self::on_claim(
			signer_pk,
            balance,
            funder_id,
			env::current_account_id(),
			NO_DEPOSIT,
			ON_CALLBACK_GAS,
		))
    }

	/// self callback checks if account was created successfully or not. If yes, refunds excess storage, sends NFTs, FTs etc..
    pub fn on_claim(&mut self, pk: PublicKey, balance: U128, funder_id: AccountId) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
		assert_eq!(env::promise_results_count(), 1, "no promise result");
        let creation_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        // Default amount to refund to be everything except balance and burnt GAS since balance was sent to new account.
        let mut amount_to_refund =  ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE - BURNT_GAS;
        
        // If not successful, the balance is added to the amount to refund since it was never transferred.
        if !creation_succeeded {
            amount_to_refund += balance.0
        } //TODO: NFTS, FTs

        Promise::new(funder_id).transfer(amount_to_refund);

        creation_succeeded
    }

    /// Returns the balance associated with given key. This is used by the NEAR wallet to display the amount of the linkdrop
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let account_data = self.accounts
            .get(&key)
            .expect("Key missing");
        (account_data.balance.0).into()
    }
}