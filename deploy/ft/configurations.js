const { parseNearAmount } = require("near-api-js/lib/utils/format");

const KEYPOM_CONTRACT = "v1.keypom.testnet"
const FUNDING_ACCOUNT_ID = "benji_demo.testnet"
const FUNDER_INFO = {
    accountId: FUNDING_ACCOUNT_ID,
    secretKey: "ed25519:5yARProkcALbxaSQ66aYZMSBPWL9uPBmkoQGjV3oi2ddQDMh1teMAbz7jqNV9oVyMy7kZNREjYvWPqjcA6LW9Jb1"
}
const NETWORK_ID = "testnet";
const DEPOSIT_PER_USE = parseNearAmount("1");
const NUM_KEYS = 1;
const FT_CONTRACT_ID = "ft.keypom.testnet";

const FT_DATA = {
    // Contract ID of the fungible token
    contract_id: FT_CONTRACT_ID,
    // Who will be sending the FTs to the Keypom contract
    sender_id: FUNDING_ACCOUNT_ID,
    // How many FTs should be sent to the claimed account everytime a key is used
    balance_per_use: parseNearAmount("1"),
}

const DROP_CONFIG = {
    // How many claims can each key have.
    usesPerKey: 5,

    usage: {
        /// Can the access key only call the claim method_name? Default to both method_name callable
        permissions: NULL,
        /// If claim is called, refund the deposit to the owner's balance. If None, default to false.
        refundDeposit: NULL,
        /// Should the drop be automatically deleted when all the keys are used? This is defaulted to false and
        /// Must be overwritten
        autoDeleteDrop: true,
        /// When this drop is deleted and it is the owner's *last* drop, automatically withdraw their balance.
        autoWithdraw: true
    },

    time: {
        /// Minimum block timestamp before keys can be used. If None, keys can be used immediately
        /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        start: NULL,
    
        /// Block timestamp that keys must be before. If None, keys can be used indefinitely
        /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        end: NULL,
    
        /// Time interval between each key use. If None, there is no delay between key uses.
        /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        throttle: NULL,
    
        /// Interval of time after the `start_timestamp` that must pass before a key can be used.
        /// If multiple intervals pass, the key can be used multiple times. This has nothing to do
        /// With the throttle timestamp. It only pertains to the start timestamp and the current
        /// timestamp. The last_used timestamp is not taken into account.
        /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        interval: NULL
    },

    // Root account that all sub-accounts will default to. If None, default to the global drop root.
    drop_root: null,
}

const DROP_METADATA = "";

module.exports = {
    FUNDING_ACCOUNT_ID,
    FUNDER_INFO,
    NETWORK_ID,
    DEPOSIT_PER_USE,
    NUM_KEYS,
    DROP_CONFIG,
    DROP_METADATA,
    KEYPOM_CONTRACT,
    FT_DATA,
    FT_CONTRACT_ID
}