const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initiateNearConnection, getFtCosts, ATTACHED_GAS_FROM_WALLET, getRecentDropId } = require("../utils/general");
const { FUNDING_ACCOUNT_ID, NETWORK_ID, NUM_KEYS, DEPOSIT_PER_USE, DROP_CONFIG, KEYPOM_CONTRACT, NFT_CONTRACT_ID, NFT_METADATA } = require("./configurations");
const { KeyPair } = require("near-api-js");
const { BN } = require("bn.js");
const { estimateRequiredDeposit } = require("keypom-js");

async function start() {
	// Initiate connection to the NEAR blockchain.
	console.log("Initiating NEAR connection");
	let near = await initiateNearConnection(NETWORK_ID);
	const nftSeries = await near.account(NFT_CONTRACT_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	const FC_DATA = {
        methods: [
			[
				{
					receiver_id: NFT_CONTRACT_ID,
					method_name: "nft_mint",
					args: "",
					attached_deposit: parseNearAmount("0.008"),
					account_id_field: "receiver_id",
					drop_id_field: "mint_id"
				}
			]
		]
    } 

	// SONKE: 4.4772845
	// Jetson: 3.684133 + 3.678263 + 1.8391315,
	// Mushrooms: 120 
	// Refraction: 
	let requiredDeposit = parseNearAmount("120")

	// let requiredDeposit = await estimateRequiredDeposit({
	// 	near,
	// 	depositPerUse: DEPOSIT_PER_USE,
	// 	numKeys: NUM_KEYS,
	// 	usesPerKey: DROP_CONFIG.uses_per_key,
	// 	attachedGasFromWallet: ATTACHED_GAS_FROM_WALLET,
	// 	fcData: FC_DATA
	// })
	console.log('requiredDeposit: ', requiredDeposit)
	
	//parseNearAmount("11");
	
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

	const dropId = Date.now().toString();

	try {
		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'create_drop', 
			{
				public_keys: pubKeys.slice(0, 100),
				deposit_per_use: DEPOSIT_PER_USE,
				fc: FC_DATA,
				drop_id: dropId,
				config: DROP_CONFIG
			}, 
			"300000000000000"
		);

		// // Loop 29 times
		// for(var i = 0; i < 29; i++) {
		// 	await fundingAccount.functionCall(
		// 		KEYPOM_CONTRACT, 
		// 		'add_keys', 
		// 		{
		// 			drop_id: dropId,
		// 			public_keys: pubKeys.slice(100 * (i+1), 100 * (i+2))
		// 		}, 
		// 		"300000000000000"
		// 	);
		// }

		// await fundingAccount.functionCall(
		// 	KEYPOM_CONTRACT, 
		// 	'add_keys', 
		// 	{
		// 		drop_id: dropId,
		// 		public_keys: pubKeys.slice(100, 200)
		// 	}, 
		// 	"300000000000000"
		// );

		// await fundingAccount.functionCall(
		// 	KEYPOM_CONTRACT, 
		// 	'add_keys', 
		// 	{
		// 		drop_id: dropId,
		// 		public_keys: pubKeys.slice(200, 300)
		// 	}, 
		// 	"300000000000000"
		// );

		// await fundingAccount.functionCall(
		// 	KEYPOM_CONTRACT, 
		// 	'add_keys', 
		// 	{
		// 		drop_id: dropId,
		// 		public_keys: pubKeys.slice(300, 400)
		// 	}, 
		// 	"300000000000000"
		// );

		// await fundingAccount.functionCall(
		// 	KEYPOM_CONTRACT, 
		// 	'add_keys', 
		// 	{
		// 		drop_id: dropId,
		// 		public_keys: pubKeys.slice(400, 420)
		// 	}, 
		// 	"300000000000000"
		// );
	} catch(e) {
		console.log('error creating drop: ', e);
	}

	try {
		await nftSeries.functionCall(
			NFT_CONTRACT_ID, 
			'create_series', 
			{
				mint_id: parseInt(dropId),
				metadata: NFT_METADATA
			}, 
			"300000000000000",
			parseNearAmount("0.02")
		);

	} catch(e) {
		console.log('error creating drop: ', e);
	}

	let curPks = {};
	for(var i = 0; i < keyPairs.length; i++) {
		curPks[keyPairs[i].publicKey.toString()] = `https://wallet.near.org/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`;
		console.log(`https://wallet.near.org/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`);
	}

	// console.log('curPks: ', curPks)
	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));
}

start();