use crate::*;

/// external and self callbacks
#[ext_contract(ext_linkdrop)]
trait ExtLinkdrop {
    fn create_account(&mut self, new_account_id: AccountId, new_public_key: PublicKey) -> Promise;
}
#[ext_contract(ext_self)]
trait ExtLinkdrop {
    fn on_account_created(&mut self, pk: PublicKey, balance: Balance, funder_id: AccountId) -> bool;
}