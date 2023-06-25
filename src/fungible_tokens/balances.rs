use crate::*;

impl InternalFTData {
    /// Add to the available balance. This should only ever be invoked in:
    /// * `ft_on_transfer` (when the transfer is successful).
    /// * `ft_resolve_batch` (when the ft_transfer failed and a refund needs to occur).
    pub fn add_to_balance_avail(&mut self, amount: &u128) {
        self.balance_avail += amount;
    }
}