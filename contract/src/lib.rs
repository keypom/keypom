use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseResult, PublicKey,
};

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct LinkDrop {
    pub linkdrop_contract: AccountId,
    pub accounts: LookupMap<PublicKey, Balance>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Accounts,
}

/// 0.02 N
const ACCESS_KEY_ALLOWANCE: u128 = 20_000_000_000_000_000_000_000;
/// can take 0.5 of access key since gas required is 6.6 times what was actually used
const NEW_ACCOUNT_BASIC_AMOUNT: u128 = 15_000_000_000_000_000_000_000;
const ON_CREATE_ACCOUNT_GAS: Gas = Gas(40_000_000_000_000);
const ON_CALLBACK_GAS: Gas = Gas(20_000_000_000_000);
/// Indicates there are no deposit for a callback for better readability.
const NO_DEPOSIT: u128 = 0;

/// external and self callbacks
#[ext_contract(ext_linkdrop)]
trait ExtLinkdrop {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}
#[ext_contract(ext_self)]
trait ExtLinkdrop {
    fn on_account_created(&mut self, pk: PublicKey) -> bool;
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
    pub fn send(&mut self, public_key: PublicKey) -> Promise {
        assert!(
            env::attached_deposit() >= ACCESS_KEY_ALLOWANCE,
            "Deposit < ACCESS_KEY_ALLOWANCE"
        );
        let pk = public_key;
        let value = self.accounts.get(&pk).unwrap_or(0);
        self.accounts.insert(
            &pk,
            &(value + env::attached_deposit() - ACCESS_KEY_ALLOWANCE),
        );
        Promise::new(env::current_account_id()).add_access_key(
            pk,
            ACCESS_KEY_ALLOWANCE,
            env::current_account_id(),
            b"claim,create_account_and_claim".to_vec(),
        )
    }

    #[payable]
    pub fn send_multiple(&mut self, public_keys: Vec<PublicKey>) {
        assert!(
            env::attached_deposit() >= ACCESS_KEY_ALLOWANCE,
            "Deposit < ACCESS_KEY_ALLOWANCE"
        );

		let current_account_id = env::current_account_id();

		let promise = env::promise_batch_create(&current_account_id);

		let len = public_keys.len() as u128;
		
		for pk in public_keys {

			env::promise_batch_action_add_key_with_function_call(
				promise, 
				&pk, 
				0, 
				ACCESS_KEY_ALLOWANCE, 
				&current_account_id, 
				b"claim,create_account_and_claim"
			);
			
			self.accounts.insert(
				&pk, 
				&(self.accounts.get(&pk).unwrap_or(0) + env::attached_deposit() / len - ACCESS_KEY_ALLOWANCE),
			);
		}

		env::promise_return(promise);
    }

	/// claiming

	fn process_claim(&mut self) -> Balance {
		assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
        let mut amount = self
            .accounts
            .remove(&env::signer_account_pk())
            .expect("Missing public key");

		Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());

		if amount == 0 {
			amount = NEW_ACCOUNT_BASIC_AMOUNT;
		}

		amount
	}

    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) -> Promise {
        assert!(
            env::is_valid_account_id(account_id.as_bytes()),
            "Invalid account id"
        );

        let amount = self.process_claim();
		
        Promise::new(account_id).transfer(amount)
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

        let amount = self.process_claim();

        ext_linkdrop::create_account(
            new_account_id,
            new_public_key,
            &self.linkdrop_contract,
            amount,
            ON_CREATE_ACCOUNT_GAS,
        ).then(ext_self::on_account_created(
			env::signer_account_pk(),
			&env::current_account_id(),
			NO_DEPOSIT,
			ON_CALLBACK_GAS,
		))
    }

    /// Returns the balance associated with given key.
    pub fn get_key_balance(&self, key: PublicKey) -> U128 {
        self.accounts
            .get(&key)
            .expect("Key missing")
            .into()
    }

	/// self callback checks if account was created successfully or not
    pub fn on_account_created(&mut self, pk:PublicKey) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
		assert_eq!(env::promise_results_count(), 1, "no promise result");
        let creation_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));
        if !creation_succeeded {
			// put access key back (was deleted before calling linkdrop contract)
            Promise::new(env::current_account_id()).add_access_key(
				pk,
				ACCESS_KEY_ALLOWANCE,
				env::current_account_id(),
				b"claim,create_account_and_claim".to_vec(),
			);
        }
        creation_succeeded
    }
}
