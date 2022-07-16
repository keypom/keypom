const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
  
let LINKDROP_PROXY_CONTRACT_ID = process.env.CONTRACT_NAME;
let FUNDING_ACCOUNT_ID = process.env.FUNDING_ACCOUNT_ID;
let LINKDROP_NEAR_AMOUNT = process.env.LINKDROP_NEAR_AMOUNT;

let OFFSET = 0.1;
let DROP_FEE = 1;
let KEY_FEE = 0.005;
let NUM_KEYS = 1;

let NETWORK_ID = "testnet";
let near;
let config;
let keyStore;

let drop_config = {
	max_claims_per_key: 2,

	start_timestamp: 0,
	// usage_interval: 6e11, // 10 minutes
	refund_if_claim: false,
	only_call_claim: false
}

let drop_metadata = {
	title: "This is a title",
	description: "This is a description"
}

/*
	Hard coding NFT contract and metadata. Change this if you want.
*/
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

	const contractAccount = await near.account(LINKDROP_PROXY_CONTRACT_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	console.log(`initializing contract for account ${LINKDROP_PROXY_CONTRACT_ID}`);
	try {
		await contractAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'new', 
			{
				linkdrop_contract: "testnet",
				owner_id: LINKDROP_PROXY_CONTRACT_ID
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	let keyPairs = [];
	let pubKeys = [];
	let viewData = {};

	console.log("BATCH Creating keypairs");
	for(var i = 0; i < NUM_KEYS; i++) {
		console.log('i: ', i);
		let keyPair = await KeyPair.fromRandom('ed25519'); 
		keyPairs.push(keyPair);   
		pubKeys.push(keyPair.publicKey.toString());   
	}
	console.log("Finished.");

	const dropId = await fundingAccount.viewFunction(
		LINKDROP_PROXY_CONTRACT_ID, 
		'get_nonce',
	);

	try {
		await fundingAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'add_to_balance', 
			{},
			"300000000000000", 
			parseNearAmount(
				((parseFloat(LINKDROP_NEAR_AMOUNT) + KEY_FEE + OFFSET) * pubKeys.length * drop_config.max_claims_per_key + DROP_FEE).toString()
			)
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	try {
		let nft_data = {};
		nft_data["nft_contract"] = NFT_CONTRACT_ID;
		nft_data["nft_sender"] = FUNDING_ACCOUNT_ID;
		nft_data["longest_token_id"] = pubKeys[0];

		await fundingAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'create_drop', 
			{
				public_keys: pubKeys,
				balance: parseNearAmount(LINKDROP_NEAR_AMOUNT),
				nft_data,
				drop_config,
				drop_metadata: JSON.stringify(drop_metadata)
			}, 
			"300000000000000"
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	try {
		const tokens = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_token_ids_for_drop',
			{
				drop_id: dropId
			}
		);
		viewData.token_ids_for_drop_before = tokens; 
		console.log('tokens: ', tokens);
	} catch(e) {
		console.log("Error getting tokens: ", e);
	}

	try {
		console.log(`minting NFT with token ID ${pubKeys[0]} on contract ${NFT_CONTRACT_ID} with receiver: ${FUNDING_ACCOUNT_ID}`);
		await fundingAccount.functionCall(
			NFT_CONTRACT_ID, 
			'nft_mint', 
			{
				token_id: pubKeys[0],
				receiver_id: FUNDING_ACCOUNT_ID,
				metadata: METADATA,
			}, 
			"300000000000000", 
			parseNearAmount('1')
		);

		console.log(`transferring NFT to linkdrop proxy contract with nft_transfer_call`);
		await fundingAccount.functionCall(
			NFT_CONTRACT_ID, 
			'nft_transfer_call', 
			{
				token_id: pubKeys[0],
				receiver_id: LINKDROP_PROXY_CONTRACT_ID,
				msg: dropId.toString(),
			}, 
			"300000000000000", 
			'1'
		);
	} catch(e) {
		console.log('error sending FTs: ', e);
	}

	try {
		const totalSupply = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'key_total_supply', 
		);
		viewData.key_total_supply = totalSupply; 
		console.log('totalSupply: ', totalSupply);

		const getKeys = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_keys'
		);
		viewData.get_keys = getKeys; 
		console.log('getKeys: ', getKeys);

		const keyInfo = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_key_information',
			{
				key: pubKeys[0]
			}
		);
		viewData.get_key_information = keyInfo; 
		console.log('keyInfo: ', keyInfo);

		const dropInfo = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_drop_information',
			{
				drop_id: dropId
			}
		);
		viewData.get_drop_information = dropInfo; 
		console.log('dropInfo: ', dropInfo);

		const keysForDrop = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_keys_for_drop',
			{
				drop_id: dropId
			}
		);
		viewData.get_keys_for_drop = keysForDrop; 
		console.log('keysForDrop: ', keysForDrop);


		const keySupplyForFunder = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'key_supply_for_funder',
			{
				account_id: FUNDING_ACCOUNT_ID
			}
		);
		viewData.key_supply_for_funder = keySupplyForFunder; 
		console.log('keySupplyForFunder: ', keySupplyForFunder);

		const dropSupplyForFunder = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'drop_supply_for_funder',
			{
				account_id: FUNDING_ACCOUNT_ID
			}
		);
		viewData.drop_supply_for_funder = dropSupplyForFunder; 
		console.log('dropSupplyForFunder: ', dropSupplyForFunder);

		const dropsForFunder = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'drops_for_funder',
			{
				account_id: FUNDING_ACCOUNT_ID
			}
		);
		viewData.drops_for_funder = dropsForFunder; 
		console.log('dropsForFunder: ', dropsForFunder);

		const tokens = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_token_ids_for_drop',
			{
				drop_id: dropId
			}
		);
		viewData.token_ids_for_drop_after = tokens; 
		console.log('tokens after: ', tokens);

		await writeFile(path.resolve(__dirname, `views-create.json`), JSON.stringify(viewData));
	} catch(e) {
		console.log('error initializing contract: ', e);
	}
	
	let curPks = {};
	for(var i = 0; i < keyPairs.length; i++) {
		curPks[keyPairs[i].publicKey.toString()] = `https://wallet.testnet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`;
		console.log(`https://wallet.testnet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`);
		console.log("Pub Key: ", keyPairs[i].publicKey.toString());
	}

	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));
}


start();