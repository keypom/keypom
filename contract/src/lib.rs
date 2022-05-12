use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseResult, PublicKey,
};

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct AccountData {
    pub funder_id: AccountId,
    pub balance: Balance,
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct LinkDrop {
    pub linkdrop_contract: AccountId,
    pub accounts: LookupMap<PublicKey, AccountData>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Accounts,
}

/// 0.03 N
const STORAGE_ALLOWANCE: u128 = 10_000_000_000_000_000_000_000;
const ACCESS_KEY_ALLOWANCE: u128 = 20_000_000_000_000_000_000_000;
const NEW_ACCOUNT_BASE: u128 = 2_840_000_000_000_000_000_000;

const ON_CREATE_ACCOUNT_GAS: Gas = Gas(40_000_000_000_000);
const ON_CALLBACK_GAS: Gas = Gas(20_000_000_000_000);
const NO_DEPOSIT: u128 = 0;

const ACCESS_KEY_METHOD_NAMES: &str = "claim,create_account_and_claim";

/// external and self callbacks
#[ext_contract(ext_linkdrop)]
trait ExtLinkdrop {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}
#[ext_contract(ext_self)]
trait ExtLinkdrop {
    fn on_account_created(&mut self, pk: PublicKey, balance: Balance, funder_id: AccountId) -> bool;
}

#[near_bindgen]
impl LinkDrop {
    #[init]
    pub fn new(linkdrop_contract: AccountId) -> Self {
        Self {
            linkdrop_contract,
            accounts: LookupMap::new(StorageKey::Accounts),
        }
    }

	pub fn set_contract(&mut self, linkdrop_contract: AccountId) {
		assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
		self.linkdrop_contract = linkdrop_contract;
	}

	/// sending (adding keys)

    #[payable]
    pub fn send(&mut self, public_key: PublicKey, balance: Balance) -> Promise {

        let attached_deposit = env::attached_deposit();

        assert!(
            attached_deposit >= ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE,
            "Deposit < ACCESS_KEY_ALLOWANCE"
        );

        if balance > 0 {
            assert!(
                attached_deposit >= balance + NEW_ACCOUNT_BASE + STORAGE_ALLOWANCE,
                "Deposit < balance + NEW_ACCOUNT_BASE + STORAGE_ALLOWANCE"
            );
        }
        
        let pk = public_key;
        assert!(self.accounts.insert(
                &pk,
                &AccountData{
                    funder_id: env::predecessor_account_id(),
                    balance,
                },
            ).is_none(),
            "Account for PublicKey exists"
        );

        Promise::new(env::current_account_id()).add_access_key(
            pk,
            ACCESS_KEY_ALLOWANCE,
            env::current_account_id(),
            ACCESS_KEY_METHOD_NAMES.to_string(),
        )
    }

    #[payable]
    pub fn send_multiple(&mut self, public_keys: Vec<PublicKey>, balance: Balance) {
	    
        let attached_deposit = env::attached_deposit();
		let len = public_keys.len() as u128;
	    
        assert!(
            attached_deposit >= (ACCESS_KEY_ALLOWANCE + STORAGE_ALLOWANCE) * len,
            "Deposit < ACCESS_KEY_ALLOWANCE"
        );

        if balance > 0 {
            assert!(
                attached_deposit >= (balance + NEW_ACCOUNT_BASE + STORAGE_ALLOWANCE) * len,
                "Deposit < balance + NEW_ACCOUNT_BASE + STORAGE_ALLOWANCE"
            );
        }

		let current_account_id = env::current_account_id();
		let promise = env::promise_batch_create(&current_account_id);
        let funder_id = env::predecessor_account_id();
		
		for pk in public_keys {

			env::promise_batch_action_add_key_with_function_call(
				promise, 
				&pk, 
				0, 
				ACCESS_KEY_ALLOWANCE, 
				&current_account_id, 
				ACCESS_KEY_METHOD_NAMES
			);
			
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

	/// claiming

	fn process_claim(&mut self) -> (PublicKey, Balance, AccountId) {
		assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );

        let signer_pk = env::signer_account_pk();

        let AccountData {
            funder_id,
            balance,
        } = self.accounts
            .remove(&signer_pk)
            .expect("Missing public key");

		Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());

		(signer_pk, balance, funder_id)
	}

    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) -> Promise {
        assert!(
            env::is_valid_account_id(account_id.as_bytes()),
            "Invalid account id"
        );

        let (_, balance, _) = self.process_claim();
		
        Promise::new(account_id).transfer(balance)
    }

    /// Create new account and and claim tokens to it.
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: AccountId,
        new_public_key: PublicKey,
    ) -> Promise {
        assert!(
            env::is_valid_account_id(new_account_id.as_bytes()),
            "Invalid account id"
        );

        let (signer_pk, balance, funder_id) = self.process_claim();

        ext_linkdrop::create_account(
            new_account_id,
            new_public_key,
            self.linkdrop_contract.clone(),
            balance + NEW_ACCOUNT_BASE,
            ON_CREATE_ACCOUNT_GAS,
        ).then(ext_self::on_account_created(
			signer_pk,
            balance,
            funder_id,
			env::current_account_id(),
			NO_DEPOSIT,
			ON_CALLBACK_GAS,
		))
    }

    /// Returns the balance associated with given key.
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        let account_data = self.accounts
            .get(&key)
            .expect("Key missing");
        (account_data.balance + NEW_ACCOUNT_BASE).into()
    }

	/// self callback checks if account was created successfully or not
    pub fn on_account_created(&mut self, pk: PublicKey, balance: Balance, funder_id: AccountId) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
		assert_eq!(env::promise_results_count(), 1, "no promise result");
        let creation_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        // if not successful, we need to add back the key and the balance
        if !creation_succeeded {
            self.accounts.insert(
                &pk,
                &AccountData{
                    funder_id,
                    balance,
                },
            );

            Promise::new(env::current_account_id()).add_access_key(
                pk,
                ACCESS_KEY_ALLOWANCE,
                env::current_account_id(),
                ACCESS_KEY_METHOD_NAMES.to_string(),
            );
        }
        creation_succeeded
    }
}
