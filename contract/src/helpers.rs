use crate::*;

impl LinkDropProxy {
    /// Asserts that the cross contract call was successful. Returns the success value
    pub(crate) fn assert_success(&mut self) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "predecessor != current"
        );
    
        assert_eq!(env::promise_results_count(), 1, "no promise result");
        matches!(env::promise_result(0), PromiseResult::Successful(_))
    }
}