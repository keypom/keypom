
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{Base58PublicKey, U128};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, Promise, PanicOnDefault, PublicKey, BorshStorageKey
};

near_sdk::setup_alloc!();

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
const NEW_ACCOUNT_BASIC_AMOUNT: u128 = 10_000_000_000_000_000_000_000;
const ON_CREATE_ACCOUNT_GAS: u64 = 40_000_000_000_000;

#[ext_contract(ext_linkdrop)]
trait ExtLinkdrop {
    fn create_account(
        &mut self,
        new_account_id: AccountId,
        new_public_key: Base58PublicKey,
    ) -> Promise;
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
    /// Allows given public key to claim sent balance.
    /// Takes ACCESS_KEY_ALLOWANCE as fee from deposit to cover account creation via an access key.
    #[payable]
    pub fn send(&mut self, public_key: Base58PublicKey) -> Promise {
        assert!(
            env::attached_deposit() >= ACCESS_KEY_ALLOWANCE,
            "Attached deposit must be greater than or equal to ACCESS_KEY_ALLOWANCE"
        );
        let pk = public_key.into();
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

    /// Claim tokens for specific account that are attached to the public key this tx is signed with.
    pub fn claim(&mut self, account_id: AccountId) -> Promise {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "Claim only can come from this account"
        );
        assert!(
            env::is_valid_account_id(account_id.as_bytes()),
            "Invalid account id"
        );
        let amount = self
            .accounts
            .remove(&env::signer_account_pk())
            .expect("Unexpected public key");
        Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());
        Promise::new(account_id).transfer(amount)
    }

    /// Create new account and and claim tokens to it.
    pub fn create_account_and_claim(
        &mut self,
        new_account_id: AccountId,
        new_public_key: Base58PublicKey,
    ) -> Promise {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "Create account and claim only can come from this account"
        );
        assert!(
            env::is_valid_account_id(new_account_id.as_bytes()),
            "Invalid account id"
        );

        let mut amount = self
            .accounts
            .remove(&env::signer_account_pk())
            .expect("Unexpected public key");

		Promise::new(env::current_account_id()).delete_key(env::signer_account_pk());

		if amount == 0 {
			amount = NEW_ACCOUNT_BASIC_AMOUNT;
		}

		ext_linkdrop::create_account(
            new_account_id,
            new_public_key,
            &self.linkdrop_contract,
            amount,
            ON_CREATE_ACCOUNT_GAS,
        )
    }

    /// Returns the balance associated with given key.
    pub fn get_key_balance(&self, key: Base58PublicKey) -> U128 {
        self.accounts.get(&key.into()).expect("Key is missing").into()
    }
}