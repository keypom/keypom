const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initiateNearConnection, getFtCosts, estimateRequiredDeposit, ATTACHED_GAS_FROM_WALLET, getRecentDropId } = require("../utils/general");
const { FUNDING_ACCOUNT_ID, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE, DROP_CONFIG, KEYPOM_CONTRACT, FT_DATA, FT_CONTRACT_ID } = require("./configurations");
const { KeyPair, keyStores, connect } = require("near-api-js");
const { BN } = require("bn.js");

async function createAccountAndClaim(privKey, newAccountId) {
	const keypomContractId = "v1.keypom.testnet";
	const network = "testnet";

	// Initiate connection to the NEAR blockchain.
	let keyStore = new keyStores.InMemoryKeyStore();

	let nearConfig = {
		networkId: network,
		keyStore,
		nodeUrl: `https://rpc.${network}.near.org`,
		walletUrl: `https://wallet.${network}.near.org`,
		helperUrl: `https://helper.${network}.near.org`,
		explorerUrl: `https://explorer.${network}.near.org`,
	};
	const nearConnection = await connect(nearConfig);

	// Create an account object for the desired account and check to see if it exists already.
	const account = await nearConnection.account(newAccountId);

	// If the call to check state fails, that means the account does not exist and we're free to
	// proceed. Otherwise, we should exit.
	try {
		await account.state();
		console.log("account already exists. Exiting early.")
		return false;
	} catch(e) {}

	// Create the keypom account object which will be used to create the new account and claim
	// The linkdrop.
	const keypomAccountObject = await nearConnection.account(keypomContractId);

	// Set the key that the keypom account object will use to sign the claim transaction
	await keyStore.setKey(network, keypomContractId, KeyPair.fromString(privKey));
	
	// Generate a new keypair for the new account that will be created.
	let newKeyPair = await KeyPair.fromRandom('ed25519');
	let newAccountPubKey = newKeyPair.publicKey.toString();

	// Create the account and claim the linkdrop.
	try {
		await keypomAccountObject.functionCall(
			keypomContractId, 
			'create_account_and_claim', 
			{
				new_account_id: newAccountId,
				new_public_key: newAccountPubKey
			},
			"100000000000000"
		);
	} catch(e) {
		console.log('error claiming linkdrop: ', e);
	}

	// Generate the auto import link for the new account
	const walletAutoImportLink = `https://wallet.testnet.near.org/auto-import-secret-key#${newAccountId}/${newKeyPair.toString()}`;
	console.log('walletAutoImportLink: ', walletAutoImportLink)
	return walletAutoImportLink;
}

createAccountAndClaim("5pod8zEoE75cDCR57dUTLPD2XqicV1fr2G3oMvTsAA4zh2w32faYUMwftCrgwEDjK2B3CNeLE5Ef2TkXwz7irHT5", "asdkajsdlaksjdlasdjkdlasdadfoo.testnet");