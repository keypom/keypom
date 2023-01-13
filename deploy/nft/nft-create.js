const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');

// NOTE: This script MUST be run on testnet and WILL NOT WORK ON MAINNET
// This is beause the chosen NFT contract for this tutorial lives on testnet.

const { initiateNearConnection, getFtCosts, estimateRequiredDeposit, ATTACHED_GAS_FROM_WALLET, getRecentDropId } = require("../utils/general");
const { FUNDING_ACCOUNT_ID, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE_NEAR, DROP_CONFIG, KEYPOM_CONTRACT, NFT_DATA, NFT_CONTRACT_ID, NFT_METADATA } = require("./configurations");
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

	try {
		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'create_drop', 
			{
				public_keys: pubKeys,
				deposit_per_use: DEPOSIT_PER_USE_NEAR,
				config: DROP_CONFIG,
				metadata: JSON.stringify(DROP_METADATA),
				nft_data: NFT_DATA
			}, 
			"300000000000000"
		);
	} catch(e) {
		console.log('error creating drop: ', e);
	}

	try {
		let dropId = await getRecentDropId(fundingAccount, FUNDING_ACCOUNT_ID, KEYPOM_CONTRACT);
		console.log('dropId: ', dropId)

		let amountToTransfer = NUM_KEYS * DROP_CONFIG.uses_per_key;
		for(var i = 0; i < amountToTransfer; i++) {
			let tokenId = `keypom-${dropId}-${i}-${FUNDING_ACCOUNT_ID}-${Date.now()}`;
			await fundingAccount.functionCall(
				NFT_CONTRACT_ID, 
				'nft_mint', 
				{
					receiver_id: FUNDING_ACCOUNT_ID,
					metadata: NFT_METADATA,
					token_id: tokenId,
				},
				"300000000000000",
				parseNearAmount("0.1")
			);

			await fundingAccount.functionCall(
				NFT_CONTRACT_ID, 
				'nft_transfer_call', 
				{
					receiver_id: KEYPOM_CONTRACT,
					token_id: tokenId,
					msg: dropId.toString()
				},
				"300000000000000",
				"1"
			);
		}
	} catch(e) {
		console.log('error sending NFTs', e);
	}
	
	let curPks = {};
	for(var i = 0; i < keyPairs.length; i++) {
		let linkdropUrl = NETWORK_ID == "testnet" ? `https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}` : `https://mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`;
		curPks[keyPairs[i].publicKey.toString()] = linkdropUrl;
		console.log(linkdropUrl);
	}

	console.log('curPks: ', curPks)
	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));
}

start();