const { parseNearAmount } = require("near-api-js/lib/utils/format");

export const CONFIGURATIONS = {
    deposit_per_use: parseNearAmount("1"),
    config: {
        // How many claims can each key have.
        uses_per_key: 1,

        // Should the drop be automatically deleted when all the keys are used? This is defaulted to false and
        // Must be overwritten
        delete_on_empty: true,

        // When this drop is deleted and it is the owner's *last* drop, automatically withdraw their balance.
        auto_withdraw: true,

        // Minimum block timestamp that keys can be used. If None, keys can be used immediately
        // Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        start_timestamp: null,

        // How often can a key be used
        // Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        throttle_timestamp: null,

        // If claim is called, refund the deposit to the owner's balance. If None, default to false.
        on_claim_refund_deposit: null,

        // Can the access key only call the claim method_name? Default to both method_name callable
        claim_permission: null,

        // Root account that all sub-accounts will default to. If None, default to the global drop root.
        drop_root: null,
    }
}