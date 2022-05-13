const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();

let LINKDROP_PROXY_CONTRACT_ID = "dev-1652411322759-29510991892447";
let FUNDING_ACCOUNT_ID = "benjiman.testnet";
let NETWORK_ID = "testnet";
let near;
let config;
let keyStore;

const METADATA = {
	"title": "Linkdrop Go Team",
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
	const contractId = LINKDROP_PROXY_CONTRACT_ID;
	await initiateNear();

	const contractAccount = await near.account(contractId);

	const sendingAccount = await near.account(FUNDING_ACCOUNT_ID);

	let state = await contractAccount.state(); 
	console.log('state before initialization: ', state);

	console.log("initializing contract");
	try {
		await contractAccount.functionCall(
			contractId, 
			'new', 
			{
				linkdrop_contract: "testnet",
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}
	
	let keyPair = await KeyPair.fromRandom('ed25519'); 
	let pubKey = keyPair.publicKey.toString();
	console.log('pubKey: ', pubKey);

	state = await contractAccount.state(); 
	console.log('state after initialization: ', state);

	console.log("sending funds as FUNDING_ACCOUNT_ID");
	try {
		await sendingAccount.functionCall(
			contractId, 
			'send', 
			{
				public_key: pubKey,
				balance: parseNearAmount("0.01")
			}, 
			"300000000000000", 
			parseNearAmount('1')
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	state = await contractAccount.state(); 
	console.log('state after send: ', state);
    
	console.log("minting and sending NFT as to public key");
	try {
		const tokenId = Date.now().toString();
		await sendingAccount.functionCall(
			'example-nft.testnet', 
			'nft_mint', 
			{
				token_id: tokenId,
				receiver_id: 'benjiman.testnet',
				token_metadata: METADATA,
			}, 
			"300000000000000", 
			parseNearAmount('1')
		);

		await sendingAccount.functionCall(
			'example-nft.testnet', 
			'nft_transfer_call', 
			{
				token_id: tokenId,
				receiver_id: contractId,
				msg: pubKey,
			}, 
			"300000000000000", 
			'1'
		);
	} catch(e) {
		console.log('error minting and sending NFTs: ', e);
	}

	state = await contractAccount.state(); 
	console.log('state after mint: ', state);

	console.log(`https://wallet.testnet.near.org/linkdrop/${contractId}/${keyPair.secretKey}`);
}

start();