const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
  
let LINKDROP_PROXY_CONTRACT_ID = "eth-toronto.keypom.near"//process.env.CONTRACT_NAME;
let FUNDING_ACCOUNT_ID = "eth-toronto.keypom.near";
let LINKDROP_NEAR_AMOUNT = "0.25"//process.env.LINKDROP_NEAR_AMOUNT;

let OFFSET = 1;
let KEY_FEE = 0.005;
let NUM_KEYS = 100;

let NETWORK_ID = "mainnet";
let near;
let keyStore;

let config = {
	uses_per_key: 500,
	//start_timestamp: 0,
	//throttle_timestamp: 1e10, // 10 seconds
	on_claim_refund_deposit: false,
	//claim_permission: 'Claim',
	drop_root: 'benjiman.testnet'
}

let NFT_CONTRACT_ID = "nft.examples.testnet";
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

	let nearConfig = {
		networkId: NETWORK_ID,
		keyStore,
		nodeUrl: "https://rpc.mainnet.near.org",
		walletUrl: "https://wallet.mainnet.near.org",
		helperUrl: "https://helper.mainnet.near.org",
		explorerUrl: "https://explorer.mainnet.near.org",
	};

	near = await connect(nearConfig);
};

async function start() {
	//deployed linkdrop proxy contract
	await initiateNear();

	if(!LINKDROP_PROXY_CONTRACT_ID) {
		const dev_account = await readFile(`neardev/dev-account`);
		LINKDROP_PROXY_CONTRACT_ID = dev_account.toString();
	}

	console.log('LINKDROP_PROXY_CONTRACT_ID: ', LINKDROP_PROXY_CONTRACT_ID);
	console.log('FUNDING_ACCOUNT_ID: ', FUNDING_ACCOUNT_ID);
	console.log('LINKDROP_NEAR_AMOUNT: ', LINKDROP_NEAR_AMOUNT);

	if(!FUNDING_ACCOUNT_ID || !LINKDROP_NEAR_AMOUNT) {
		throw "must specify funding account and linkdrop near amount";
	}

	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	let keyPairs = [];
	let pubKeys = [];

	console.log("BATCH Creating keypairs");
	for(var i = 0; i < NUM_KEYS; i++) {
		console.log('i: ', i);
		let keyPair = await KeyPair.fromRandom('ed25519'); 
		keyPairs.push(keyPair);   
		pubKeys.push(keyPair.publicKey.toString());   
	}
	console.log("Finished.");

	let dropId = await fundingAccount.viewFunction(
		LINKDROP_PROXY_CONTRACT_ID, 
		'get_next_drop_id',
	);
	
	console.log(`Next drop ID: ${dropId}. Cur drop ID: ${dropId - 1}`);
	dropId -= 1;

	try {
		await fundingAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'add_keys', 
			{
				public_keys: pubKeys,
				drop_id: dropId
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	let curPksBuff = await readFile(path.resolve(__dirname, `pks.json`));
	let curPks = JSON.parse(curPksBuff);
	for(var i = 0; i < keyPairs.length; i++) {
		curPks[keyPairs[i].publicKey.toString()] = `https://wallet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`;
		console.log(`https://wallet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`);
		console.log("Pub Key: ", keyPairs[i].publicKey.toString());
	}

	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));

}


start();