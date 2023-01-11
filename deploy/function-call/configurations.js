const { parseNearAmount } = require("near-api-js/lib/utils/format");

const KEYPOM_CONTRACT = "beta.keypom.testnet"
const FUNDING_ACCOUNT_ID = "benji_demo.testnet"
const FUNDER_INFO = {
    accountId: FUNDING_ACCOUNT_ID,
    secretKey: "ed25519:5yARProkcALbxaSQ66aYZMSBPWL9uPBmkoQGjV3oi2ddQDMh1teMAbz7jqNV9oVyMy7kZNREjYvWPqjcA6LW9Jb1"
}
const NETWORK_ID = "testnet";
const DEPOSIT_PER_USE_NEAR = 1;
const NUM_KEYS = 420;
const NFT_CONTRACT_ID = "nft.keypom.testnet";


const FC_DATA = {
    methods: [
		[{
			receiverId: "dev-1664052531433-97566156431683",
			methodName: "nft_mint",
			args: JSON.stringify({
				"foo": "bar",
				"keypom_args": {
					"account_id_field": "receiver_id",
					"drop_id_field" : "mint_id"
				}
			}),
			attachedDeposit: parseNearAmount("1"),
			accountIdField: "receiver_id",
			dropIdField: "mint_id"
		}]
	]
}

const DROP_CONFIG = {
    // How many claims can each key have.
    usesPerKey: 1,

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
    dropRoot: null,
}

const DROP_METADATA = "";

module.exports = {
    FUNDING_ACCOUNT_ID,
    FUNDER_INFO,
    NETWORK_ID,
    DEPOSIT_PER_USE_NEAR,
    NUM_KEYS,
    DROP_CONFIG,
    DROP_METADATA,
    FC_DATA,
    KEYPOM_CONTRACT,
    NFT_CONTRACT_ID,
    NFT_METADATA
}