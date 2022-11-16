const { parseNearAmount } = require("near-api-js/lib/utils/format");

const KEYPOM_CONTRACT = "beta.keypom.testnet"
const FUNDING_ACCOUNT_ID = "benjiman.testnet";
const NETWORK_ID = "testnet";
const DEPOSIT_PER_USE = parseNearAmount("0.003");
const NUM_KEYS = 420;
const NFT_CONTRACT_ID = "nft.keypom.testnet";

const NFT_METADATA = {
    "media": "bafybeiblargpzhwxgmbzzci6n6oubfhcw33cdqb4uqx62sxrvf5biwcszi",
    "title": "OG 420 SpliffDAO Validator",
    "description": "IF YOU GOT THIS, this means you Proof of Sesh’d‍ with a SpliffDAO ‍OG Validator. You are now an on-chain OG. You are one of the first 420 SpliffDAO OG Validators on Solana (the first blockchain for SpliffDAO & BluntDAO). Now you can onboard other into Spliff DAO by smoking a spliff with them in real life via #ProofOfSesh. SpliffDAO.com. A Spliff = joint with tobacco in it. SpliffDAO is brought to you by BluntDAO (bluntdao.org) as part of the decentralized IRL movement to onboard the next million people into Web3, 1 sesh at a time. ",
    "copies": 420,
}

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