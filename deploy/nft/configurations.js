const { parseNearAmount } = require("near-api-js/lib/utils/format");

const KEYPOM_CONTRACT = "v1.keypom.testnet"
const FUNDING_ACCOUNT_ID = "benjiman.testnet";
const NETWORK_ID = "testnet";
const DEPOSIT_PER_USE = parseNearAmount("1");
const NUM_KEYS = 2;
const NFT_CONTRACT_ID = "nft.examples.testnet";

const NFT_METADATA = {
    title: "My Keypom NFT",
    description: "Keypom is lit fam",
    media: "https://bafybeiftczwrtyr3k7a2k4vutd3amkwsmaqyhrdzlhvpt33dyjivufqusq.ipfs.dweb.link/goteam-gif.gif",
}

const NFT_DATA = {
    // NFT Contract Id that the tokens will come from
    contract_id: NFT_CONTRACT_ID,
    // Who will be sending the NFTs to the Keypom contract
    sender_id: FUNDING_ACCOUNT_ID,
}

const DROP_CONFIG = {
    // How many claims can each key have.
    uses_per_key: 3,

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

const DROP_METADATA = "";

module.exports = {
    FUNDING_ACCOUNT_ID,
    NETWORK_ID,
    DEPOSIT_PER_USE,
    NUM_KEYS,
    DROP_CONFIG,
    DROP_METADATA,
    KEYPOM_CONTRACT,
    NFT_DATA,
    NFT_CONTRACT_ID,
    NFT_METADATA
}