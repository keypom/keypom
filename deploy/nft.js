const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
  
let LINKDROP_PROXY_CONTRACT_ID = process.env.LINKDROP_PROXY_CONTRACT_ID;
let FUNDING_ACCOUNT_ID = process.env.FUNDING_ACCOUNT_ID;
let LINKDROP_NEAR_AMOUNT = process.env.LINKDROP_NEAR_AMOUNT;

let NETWORK_ID = "testnet";
let near;
let config;
let keyStore;

/*
	Hard coding NFT contract and metadata. Change this if you want.
*/
let NFT_CONTRACT_ID = "example-nft.testnet";
const METADATA = {
	"title": "Linkdropped Go Team NFT",
	"description": "Testing Linkdrop NFT Go Team Token",
	"media": "https://bafybeiftczwrtyr3k7a2k4vutd3amkwsmaqyhrdzlhvpt33dyjivufqusq.ipfs.dweb.link/goteam-gif.gif",
	"media_hash": null,
	"copies": 10000,
	"issued_at": null,
	"expires_at": null,
	"starts_at": null,
	"updated_at": null,
	"extra": null,
	"reference": null,
	"reference_hash": null
};

// set up near
const initiateNear = async () => {
	const CREDENTIALS_DIR = ".near-credentials";

	const credentialsPath = (await path).join(homedir, CREDENTIALS_DIR);
	(await path).join;
	keyStore = new keyStores.UnencryptedFileSystemKeyStore(credentialsPath);

	config = {
		networkId: NETWORK_ID,
		keyStore,
		nodeUrl: "https://rpc.testnet.near.org",
		walletUrl: "https://wallet.testnet.near.org",
		helperUrl: "https://helper.testnet.near.org",
		explorerUrl: "https://explorer.testnet.near.org",
	};

	near = await connect(config);
};

async function start() {
	//deployed linkdrop proxy contract
	await initiateNear();

	if(!LINKDROP_PROXY_CONTRACT_ID || !FUNDING_ACCOUNT_ID || !LINKDROP_NEAR_AMOUNT) {
		throw "must specify proxy contract ID, funding account ID and linkdrop $NEAR amount";
	}

	const contractAccount = await near.account(LINKDROP_PROXY_CONTRACT_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	console.log(`initializing contract for account ${LINKDROP_PROXY_CONTRACT_ID}`);
	try {
		await contractAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'new', 
			{
				linkdrop_contract: "testnet",
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}
	
	console.log("generating keypair.");
	let keyPair = await KeyPair.fromRandom('ed25519'); 
	let pubKey = keyPair.publicKey.toString();
	console.log('pubKey: ', pubKey);

	console.log(`sending ${LINKDROP_NEAR_AMOUNT} $NEAR as ${FUNDING_ACCOUNT_ID}`);
	try {
		await fundingAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'send', 
			{
				public_key: pubKey,
				balance: parseNearAmount(LINKDROP_NEAR_AMOUNT)
			}, 
			"300000000000000", 
			parseNearAmount((parseFloat(LINKDROP_NEAR_AMOUNT) + 1).toString())
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	try {
		const tokenId = Date.now().toString();
		console.log(`minting NFT with token ID ${tokenId} on contract ${tokenId} with receiver: ${FUNDING_ACCOUNT_ID}`);
		
		await fundingAccount.functionCall(
			NFT_CONTRACT_ID, 
			'nft_mint', 
			{
				token_id: tokenId,
				receiver_id: FUNDING_ACCOUNT_ID,
				token_metadata: METADATA,
			}, 
			"300000000000000", 
			parseNearAmount('1')
		);

		console.log(`transferring NFT to linkdrop proxy contract with nft_transfer_call`);
		await fundingAccount.functionCall(
			NFT_CONTRACT_ID, 
			'nft_transfer_call', 
			{
				token_id: tokenId,
				receiver_id: LINKDROP_PROXY_CONTRACT_ID,
				msg: pubKey,
			}, 
			"300000000000000", 
			'1'
		);
	} catch(e) {
		console.log('error minting and sending NFTs: ', e);
	}
    
	console.log(`https://wallet.testnet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPair.secretKey}`);
}

start();