use crate::*;

impl InternalFTData {
    /// Add to the available balance. This should only ever be invoked in:
    /// * `ft_on_transfer` (when the transfer is successful).
    /// * `ft_resolve_batch` (when the ft_transfer failed and a refund needs to occur).
    pub fn add_to_balance_avail(&mut self, amount: &U128) {
        self.balance_avail.0 += amount.0;
    }

    /// Check whether or not there's enough balance to transfer a given amount.
    pub fn enough_balance(&self, transfer_amount: &U128) -> bool {
        self.balance_avail.0 >= transfer_amount.0
    }
}