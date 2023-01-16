const { parseNearAmount } = require("near-api-js/lib/utils/format");

const KEYPOM_CONTRACT = "v1-3.keypom.testnet"
const FUNDING_ACCOUNT_ID = "minqi.testnet"
const FUNDER_INFO = {
    accountId: FUNDING_ACCOUNT_ID,
    secretKey: "ed25519:3hsCWpjczaPoNejnC2A1McGvnJQipAJUDmo6tEZ6XH6qwxfxTLkpQ8hMNG3jxg1zXEe5Ke2qoqUq76jJpeNKxaMa"
}
// NOTE: This script MUST be run on testnet and WILL NOT WORK ON MAINNET
// This is beause the chosen NFT contract for this tutorial lives on testnet.
const NETWORK_ID = "testnet";
const DEPOSIT_PER_USE_NEAR = 1;
const NUM_KEYS = 2;
const NFT_CONTRACT_ID = 'nft.examples.testnet';

const NFT_METADATA = {
    title: "My Keypom NFT",
    description: "Keypom is lit fam :D",
    media: "https://bafybeiftczwrtyr3k7a2k4vutd3amkwsmaqyhrdzlhvpt33dyjivufqusq.ipfs.dweb.link/goteam-gif.gif",
}

const NFT_DATA = {
    // NFT Contract Id that the tokens will come from
    contractId: NFT_CONTRACT_ID,
    // Who will be sending the NFTs to the Keypom contract
    senderId: FUNDING_ACCOUNT_ID,
    // List of tokenIDs
    // tokenIds: ["1.0.6", "1.0.7"]
    tokenIds: ["1abc"]
}
//USED TO HAVE 2 OBJS
// const NFT_DATA_OWNED = {
//     // NFT Contract Id that the tokens will come from
//     contract_id: NFT_CONTRACT_ID,
//     // Who will be sending the NFTs to the Keypom contract
//     sender_id: FUNDING_ACCOUNT_ID,
//     //list of tokenIDs
//     tokenIds: []
// }
    

const DROP_CONFIG = {
    // How many claims can each key have.
    usesPerKey: 1,

    usage: {
        /// Can the access key only call the claim method_name? Default to both method_name callable
        permissions: null,
        /// If claim is called, refund the deposit to the owner's balance. If None, default to false.
        refundDeposit: null,
        /// Should the drop be automatically deleted when all the keys are used? This is defaulted to false and
        /// Must be overwritten
        autoDeleteDrop: true,
        /// When this drop is deleted and it is the owner's *last* drop, automatically withdraw their balance.
        autoWithdraw: true
    },

    time: {
        /// Minimum block timestamp before keys can be used. If None, keys can be used immediately
        /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        start: null,
    
        /// Block timestamp that keys must be before. If None, keys can be used indefinitely
        /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        end: null,
    
        /// Time interval between each key use. If None, there is no delay between key uses.
        /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        throttle: null,
    
        /// Interval of time after the `start_timestamp` that must pass before a key can be used.
        /// If multiple intervals pass, the key can be used multiple times. This has nothing to do
        /// With the throttle timestamp. It only pertains to the start timestamp and the current
        /// timestamp. The last_used timestamp is not taken into account.
        /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
        interval: null
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
    KEYPOM_CONTRACT,
    NFT_DATA,
    NFT_CONTRACT_ID,
    NFT_METADATA
}