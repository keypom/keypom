const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initiateNearConnection, getFtCosts, estimateRequiredDeposit, ATTACHED_GAS_FROM_WALLET } = require("../utils/general");
const { FUNDING_ACCOUNT_ID, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE_NEAR, DROP_CONFIG, KEYPOM_CONTRACT } = require("./configurations");
const { KeyPair } = require("near-api-js");
const { parse } = require("url");

async function start() {
	// Initiate connection to the NEAR blockchain.
	console.log("Initiating NEAR connection");
	let near = await initiateNearConnection(NETWORK_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);
	
	// Keep track of an array of the keyPairs we create
	let keyPairs = [];
	// Keep track of the public keys to pass into the contract
	let pubKeys = [];
	console.log("Creating keypairs");
	for(var i = 0; i < NUM_KEYS; i++) {
		let keyPair = await KeyPair.fromRandom('ed25519'); 
		keyPairs.push(keyPair);   
		pubKeys.push(keyPair.publicKey.toString());   
	}

	// Create drop with pub keys, deposit_per_use, a default drop_config and metadata
	try {
		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'create_drop', 
			{
				public_keys: pubKeys,
				deposit_per_use: parseNearAmount(DEPOSIT_PER_USE_NEAR.toString()),
				config: {
					uses_per_key: DROP_CONFIG.usesPerKey,
					time: {
						start: DROP_CONFIG.time.start,
						end: DROP_CONFIG.time.end,
						throttle: DROP_CONFIG.time.throttle,
						interval: DROP_CONFIG.time.interval
					},
					usage: {
						permissions: DROP_CONFIG.usage.permissions,
						refund_deposit: DROP_CONFIG.usage.refundDeposit,
						auto_delete_drop:DROP_CONFIG.usage.autoDeleteDrop,
						auto_withdraw: DROP_CONFIG.usage.autoWithdraw
					
					},
					root_account_id: DROP_CONFIG.dropRoot
					
				},
				metadata: JSON.stringify(DROP_METADATA)
			}, 
			"300000000000000",
			// Change this deposit value to whatever is needed to fund your drop; this will be added to your balance...?
			parseNearAmount("2"),
		);

	} catch(e) {
		console.log('error creating drop: ', e);
	}

	console.log("checking to see if drop creation was successful, Keypom smart contract will panic if drop does not exist");
	await fundingAccount.functionCall(
		KEYPOM_CONTRACT, 
		'get_drop_information', 
		{
			key: pubKeys[0],
		}
	);
	
	//manually create linkdrops links with each key in the drop and index with pk
	let curPks = {};
	for(var i = 0; i < keyPairs.length; i++) {
		let linkdropUrl = NETWORK_ID == "testnet" ? `https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}` : `https://mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`;
		curPks[keyPairs[i].publicKey.toString()] = linkdropUrl;
		console.log(linkdropUrl);
	}

	//write file of all pk's and their respective linkdrops
	console.log('curPks: ', curPks)
	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));
}

start();