const { parseNearAmount } = require("near-api-js/lib/utils/format");

const KEYPOM_CONTRACT = "beta.keypom.testnet"
const FUNDING_ACCOUNT_ID = "benjiman.testnet";
const NETWORK_ID = "testnet";
const DEPOSIT_PER_USE = parseNearAmount("0.003");
const NUM_KEYS = 420;
const NFT_CONTRACT_ID = "nft.keypom.testnet";

const DROP_CONFIG = {
    // How many claims can each key have.
    uses_per_key: 1,
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
    NFT_CONTRACT_ID,
    NFT_METADATA
}