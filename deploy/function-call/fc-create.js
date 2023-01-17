const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');

// NOTE: This script MUST be run on testnet and WILL NOT WORK ON MAINNET
// This is beause the chosen NFT contract for this tutorial lives on testnet.

const { initiateNearConnection, getFtCosts, estimateRequiredDeposit, ATTACHED_GAS_FROM_WALLET, getRecentDropId } = require("../utils/general");
const { FUNDER_INFO, NUM_KEYS, DROP_CONFIG, NETWORK_ID, KEYPOM_CONTRACT, FUNDING_ACCOUNT_ID,NFT_METADATA, FC_DATA, DROP_METADATA, DEPOSIT_PER_USE_NEAR} = require("./configurations");
const { KeyPair } = require("near-api-js");
const { BN } = require("bn.js");

async function start() {
	// Initiate connection to the NEAR blockchain.
	console.log("Initiating NEAR connection");
	let near = await initiateNearConnection(NETWORK_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	let requiredDeposit = await estimateRequiredDeposit({
		near,
		depositPerUse: DEPOSIT_PER_USE_NEAR,
		numKeys: NUM_KEYS,
		usesPerKey: DROP_CONFIG.usesPerKey,
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

	// Create FC drop with pubkkeys from above and fc data
	try {
		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'create_drop', 
			{
				public_keys: pubKeys,
				deposit_per_use: parseNearAmount(DEPOSIT_PER_USE_NEAR.toString()),
				fc: {
					methods: [[{
						receiver_id: FC_DATA.methods[0][0].receiverId,
						method_name: FC_DATA.methods[0][0].methodName,
						args: FC_DATA.methods[0][0].args,
						attached_deposit: FC_DATA.methods[0][0].attachedDeposit,
						account_id_field: FC_DATA.methods[0][0].accountIdField
					}]]
				},
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
			parseNearAmount("2")
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
	

	let curPks = {};
	for(var i = 0; i < keyPairs.length; i++) {
		let linkdropUrl = NETWORK_ID == "testnet" ? `https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}` : `https://mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`;
		curPks[keyPairs[i].publicKey.toString()] = linkdropUrl;
		console.log(linkdropUrl);
	}

	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));
}


start();