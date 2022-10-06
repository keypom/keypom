const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initiateNearConnection, getFtCosts, estimateRequiredDeposit, ATTACHED_GAS_FROM_WALLET } = require("../utils/general");
const { FUNDING_ACCOUNT_ID, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE, DROP_CONFIG, KEYPOM_CONTRACT } = require("./configurations");
const { KeyPair } = require("near-api-js");

async function start() {
	// Initiate connection to the NEAR blockchain.
	console.log("Initiating NEAR connection");
	let near = await initiateNearConnection(NETWORK_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	let requiredDeposit = await estimateRequiredDeposit(
		near,
		DEPOSIT_PER_USE,
		NUM_KEYS,
		DROP_CONFIG.uses_per_key,
		ATTACHED_GAS_FROM_WALLET,
	)
	
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

	try {
		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'add_to_balance', 
			{},
			"300000000000000", 
			requiredDeposit.toString()
		);
	} catch(e) {
		console.log('error adding to balance: ', e);
	}

	try {
		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'create_drop', 
			{
				public_keys: pubKeys,
				deposit_per_use: DEPOSIT_PER_USE,
				config: DROP_CONFIG,
				metadata: JSON.stringify(DROP_METADATA)
			}, 
			"300000000000000"
		);
	} catch(e) {
		console.log('error creating drop: ', e);
	}
	
	let curPks = {};
	for(var i = 0; i < keyPairs.length; i++) {
		curPks[keyPairs[i].publicKey.toString()] = `https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`;
		console.log(`https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`);
	}

	console.log('curPks: ', curPks)
	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));
}

start();