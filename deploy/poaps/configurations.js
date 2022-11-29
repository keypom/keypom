const { parseNearAmount } = require("near-api-js/lib/utils/format");

const KEYPOM_CONTRACT = "v1-3.keypom.testnet"
const FUNDING_ACCOUNT_ID = "benjiman.testnet";
const NETWORK_ID = "testnet";
//const DEPOSIT_PER_USE = parseNearAmount("0.01");
const DEPOSIT_PER_USE = parseNearAmount("0.003");
const NUM_KEYS = 1;
const NFT_CONTRACT_ID = "nft.keypom.testnet";

// Jetson
// const NFT_METADATA = {
//     "media": "bafybeiax2n6wtil67a6w5qcdm4jwnnxb34ujy2ldgbbanpaoudv7jvgizu",
//     "title": "R3alyfe Life Access NFT Founding Member",
//     "description": "This NFT Acts as a Life time Entry pass to ALL things R3al , this will grant you access and invitation to all COMMUNITY Offerings ( Investment deals, Open Events, Open Oppourtunites  )",
//     "copies": 250,
// }

// Sonke Talk
const NFT_METADATA = {
    "media": "bafkreifgjnfpzjpfijndodzqw262z2xrec3qjfut5nyoekbysozwwpqakq",
    "title": "Danny Daze: BLUE - Premier Attendance NFT",
    "description": "Default Description",
    "copies": 500,
}

const DROP_CONFIG = {
    // How many claims can each key have.
    uses_per_key: 500,
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