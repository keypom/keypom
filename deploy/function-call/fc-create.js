const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initiateNearConnection, getFtCosts, estimateRequiredDeposit, ATTACHED_GAS_FROM_WALLET, getRecentDropId } = require("../utils/general");
const { FUNDER_INFO, NUM_KEYS, DROP_CONFIG, NETWORK_ID, KEYPOM_CONTRACT, FUNDING_ACCOUNT_ID, DEPOSIT_PER_USE,NFT_METADATA, FC_DATA, DROP_METADATA} = require("./configurations");
const { KeyPair } = require("near-api-js");
const { BN } = require("bn.js");

async function start() {
	// Initiate connection to the NEAR blockchain.
	console.log("Initiating NEAR connection");
	let near = await initiateNearConnection(NETWORK_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	let requiredDeposit = await estimateRequiredDeposit({
		near,
		depositPerUse: DEPOSIT_PER_USE,
		numKeys: NUM_KEYS,
		usesPerKey: DROP_CONFIG.uses_per_key,
		attachedGas: ATTACHED_GAS_FROM_WALLET,
})

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

	//create FC drop with pubkkeys from above and fc data
	try {
		await fundingAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'create_drop', 
			{
				public_keys: pubKeys,
				deposit_per_use: DEPOSIT_PER_USE,
				fc_data: FC_DATA,
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
		curPks[keyPairs[i].publicKey.toString()] = `https://wallet.testnet.near.org/linkdrop/${KEYPOM_CONTRACT_ID}/${keyPairs[i].secretKey}`;
		console.log(`https://wallet.testnet.near.org/linkdrop/${KEYPOM_CONTRACT_ID}/${keyPairs[i].secretKey}`);
		console.log("Pub Key: ", keyPairs[i].publicKey.toString());
	}

	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));
}


start();