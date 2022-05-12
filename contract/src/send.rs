use crate::*;

#[near_bindgen]
impl LinkDropProxy {
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
                    token_contract: None,
                    nft_id: None,
                    token_sender: None
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
                        token_contract: None,
                        nft_id: None,
                        token_sender: None
                    },
                ).is_none(),
                "Account for PublicKey exists"
            );
        }

        env::promise_return(promise);
    }
}