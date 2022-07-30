const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
  
let LINKDROP_PROXY_CONTRACT_ID = process.env.CONTRACT_NAME;
let FUNDING_ACCOUNT_ID = process.env.FUNDING_ACCOUNT_ID;
let LINKDROP_NEAR_AMOUNT = process.env.LINKDROP_NEAR_AMOUNT;

let OFFSET = 1;
let KEY_FEE = 0.005;
let NUM_KEYS = 1;

let NETWORK_ID = "testnet";
let near;
let keyStore;

let config = {
	uses_per_key: 2,
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
		nodeUrl: "https://rpc.testnet.near.org",
		walletUrl: "https://wallet.testnet.near.org",
		helperUrl: "https://helper.testnet.near.org",
		explorerUrl: "https://explorer.testnet.near.org",
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

	dropId -= 1;
	
	console.log('dropId: ', dropId);

	try {
		await fundingAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'add_to_balance', 
			{},
			"300000000000000", 
			parseNearAmount(
				((parseFloat(LINKDROP_NEAR_AMOUNT) + KEY_FEE + OFFSET + 1) * pubKeys.length * config.uses_per_key || 1).toString()
			)
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

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

	try {
		console.log(`minting NFT with token ID ${pubKeys[0]} on contract ${NFT_CONTRACT_ID} with receiver_id: ${FUNDING_ACCOUNT_ID}`);
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
		let viewData = {};
		const totalSupply = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_key_total_supply', 
		);
		viewData.get_key_total_supply = totalSupply; 
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
			'get_key_supply_for_owner',
			{
				account_id: FUNDING_ACCOUNT_ID
			}
		);
		viewData.get_key_supply_for_owner = keySupplyForFunder; 
		console.log('keySupplyForFunder: ', keySupplyForFunder);

		const dropSupplyForFunder = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_drop_supply_for_owner',
			{
				account_id: FUNDING_ACCOUNT_ID
			}
		);
		viewData.get_drop_supply_for_owner = dropSupplyForFunder; 
		console.log('dropSupplyForFunder: ', dropSupplyForFunder);

		const dropsForFunder = await fundingAccount.viewFunction(
			LINKDROP_PROXY_CONTRACT_ID, 
			'get_drops_for_owner',
			{
				account_id: FUNDING_ACCOUNT_ID
			}
		);
		viewData.get_drops_for_owner = dropsForFunder; 
		console.log('dropsForFunder: ', dropsForFunder);

		await writeFile(path.resolve(__dirname, `views-add.json`), JSON.stringify(viewData));
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	let curPksBuff = await readFile(path.resolve(__dirname, `pks.json`));
	let curPks = JSON.parse(curPksBuff);
	for(var i = 0; i < keyPairs.length; i++) {
		curPks[keyPairs[i].publicKey.toString()] = `https://wallet.testnet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`;
		console.log(`https://wallet.testnet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`);
		console.log("Pub Key: ", keyPairs[i].publicKey.toString());
	}

	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));

}


start();