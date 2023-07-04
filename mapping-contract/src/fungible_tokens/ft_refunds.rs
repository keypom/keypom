use near_sdk::{ext_contract, PromiseResult};

use crate::*;

/// Minimum Gas required to perform a simple transfer of fungible tokens.
/// 5 TGas
const MIN_GAS_FOR_FT_TRANSFER: Gas = Gas(5_000_000_000_000);
/// Minimum Gas required to resolve the batch of promises for transferring the FTs and registering the user.
/// 5 TGas
const MIN_GAS_FOR_RESOLVE_REFUND: Gas = Gas(5_000_000_000_000);

/// FT contract
#[ext_contract(ext_ft_contract)]
trait ExtFTContract {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[near_bindgen]
impl Keypom {
    #[private]
    pub fn ft_resolve_refund(
        &mut self, 
        drop_id: DropId, 
        asset_id: AssetId, 
        refund_to: AccountId,
        tokens_to_transfer: Balance, 
        near_refund_amount: Balance
    ) -> bool {
        let transfer_succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));

        // Everything went well so we return true since the keys registered have already been decremented
        // At this point, we should also refund their user balance with the $NEAR from registration
        if transfer_succeeded {
            near_sdk::log!(
                "Successfully refunded {} FTs for drop ID {}",
                tokens_to_transfer,
                drop_id,
            );

            self.internal_modify_user_balance(&refund_to, near_refund_amount, false);
            return true;
        }

        // Transfer failed so we need to increment the uses registered and return false
        let mut drop = self.drop_by_id.get(&drop_id).expect("no drop for ID");
        let mut internal_asset = drop.asset_by_id.get(&asset_id).expect("no asset for ID");
        
        // ensure asset is FT and then increment the tokens to transfer again
        if let InternalAsset::ft(ref mut ft_asset) = internal_asset {
            ft_asset.add_to_balance_avail(&tokens_to_transfer);
            drop.asset_by_id.insert(&asset_id, &internal_asset);
        } else {
            panic!("asset is not FT");
        }

        self.drop_by_id.insert(&drop_id, &drop);

        //near_sdk::log!("Unsuccessful refund for drop ID {}. {} keys added back as registered. Returning false.", drop_id, num_to_refund);
        false
    }
}

impl InternalFTData {
    /// Automatically refund a claim for fungible tokens
    /// This should refund the FTs & any storage deposits.
    pub fn ft_refund(
        &mut self, 
        drop_id: &DropId, 
        tokens_to_transfer: Balance, 
        refund_to: &AccountId
    ) {
        require!(self.enough_balance(&tokens_to_transfer), "not enough balance to transfer");
        
        // Temporarily decrease the available balance
        // Once the FTs are transferred, we will check whether it failed and refund there
        // Possible re-entrancy attack if we don't do this
        self.balance_avail -= tokens_to_transfer;

        // All FTs can be refunded at once. Funder responsible for registering themselves
        ext_ft_contract::ext(self.contract_id.clone())
            // Call ft transfer with 1 yoctoNEAR. 1/2 unspent GAS will be added on top
            .with_attached_deposit(1)
            .with_static_gas(MIN_GAS_FOR_FT_TRANSFER)
            .ft_transfer(
                refund_to.clone(),
                U128(tokens_to_transfer),
                Some("Keypom Refund".to_string()),
            )
            // We then resolve the promise and call ft_resolve_refund on our own contract
            .then(
                // Call resolve refund with the min GAS and no attached_deposit. 1/2 unspent GAS will be added on top
                Keypom::ext(env::current_account_id())
                    .with_static_gas(MIN_GAS_FOR_RESOLVE_REFUND)
                    .ft_resolve_refund(
                        drop_id.to_string(), 
                        self.contract_id.to_string(), 
                        refund_to.clone(),
                        tokens_to_transfer,
                        self.registration_cost
                    )
            )
            .as_return();
    }
}