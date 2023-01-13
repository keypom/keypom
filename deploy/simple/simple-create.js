const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initiateNearConnection, getFtCosts, estimateRequiredDeposit, ATTACHED_GAS_FROM_WALLET } = require("../utils/general");
const { FUNDING_ACCOUNT_ID, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE_NEAR, DROP_CONFIG, KEYPOM_CONTRACT } = require("./configurations");
const { KeyPair } = require("near-api-js");

async function start() {
	// Initiate connection to the NEAR blockchain.
	console.log("Initiating NEAR connection");
	let near = await initiateNearConnection(NETWORK_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	//get required deposit
	// let requiredDeposit = await estimateRequiredDeposit(
	// 	near,
	// 	DEPOSIT_PER_USE_NEAR,
	// 	NUM_KEYS,
	// 	DROP_CONFIG.uses_per_key,
	// 	ATTACHED_GAS_FROM_WALLET,
	// )
	
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

	//using benjiman.testnet to add requiredDeposit to keypom balance
	//switch to attached deposit, no more add to balance
	// try {
	// 	await fundingAccount.functionCall(
	// 		KEYPOM_CONTRACT, 
	// 		'add_to_balance', 
	// 		{},
	// 		"300000000000000", 
	// 		requiredDeposit.toString()
	// 	);
	// } catch(e) {
	// 	console.log('error adding to balance: ', e);
	// }

	//create drop with pub keys, deposit_per_use, a default drop_config and metadata
	try {
		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'create_drop', 
			{
				public_keys: pubKeys,
				deposit_per_use: DEPOSIT_PER_USE_NEAR,
				config: DROP_CONFIG,
				metadata: JSON.stringify(DROP_METADATA)
			}, 
			"300000000000000"
		);
	} catch(e) {
		console.log('error creating drop: ', e);
	}
	
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