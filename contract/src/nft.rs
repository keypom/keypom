use crate::*;

pub trait NonFungibleTokenReceiver {
    fn nft_on_transfer(
        &mut self,
        token_id: String,
        msg: PublicKey,
    ) -> PromiseOrValue<bool>;
}

#[near_bindgen]
impl NonFungibleTokenReceiver for LinkDropProxy {
    fn nft_on_transfer(
        &mut self,
        token_id: String,
        msg: PublicKey,
    ) -> PromiseOrValue<bool> {
        assert!(token_id.len() <= 256, "Contract cannot accept token IDs of length greater than 256 bytes");

        let contract_id = env::predecessor_account_id();

        // No need to assert that the funder is the sender since we don't wanna enforce anything unnecessary.
        // All that matters is we've received the token and that the token belongs to some public key.
        let AccountData {
            funder_id: _,
            balance,
            token_contract,
            nft_id: _,
        } = self.accounts
            .get(&msg)
            .expect("Missing public key");

        // Ensure that both the token contract is none since we only store one set of NFT / FT data.
        assert!(token_contract.is_none(), "PK must have no external token contract.");

        //insert the NFT token ID and token contract back into the map
        self.accounts.insert(
            &msg,
            &AccountData{
                funder_id: env::predecessor_account_id(),
                balance: balance,
                nft_id: Some(token_id),
                token_contract: Some(contract_id)
            },
        );

        // Everything went well and we don't need to return the token.
        PromiseOrValue::Value(false)
    }
}
