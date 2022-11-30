const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
const { initiateNearConnection, getFtCosts, estimateRequiredDeposit, ATTACHED_GAS_FROM_WALLET, getRecentDropId } = require("../utils/general");
const { FUNDING_ACCOUNT_ID, NETWORK_ID, NUM_KEYS, DROP_METADATA, DEPOSIT_PER_USE, DROP_CONFIG, KEYPOM_CONTRACT, FT_DATA, FT_CONTRACT_ID } = require("./configurations");
const { KeyPair, keyStores, connect } = require("near-api-js");
const { BN } = require("bn.js");
const { generateSeedPhrase } = require("near-seed-phrase");
var createHash = require('create-hash')

async function createAccountAndClaim(privKey, newAccountId, pinCode) {
	const network = "testnet";
	const keypomContractId = network == "testnet" ? "v1-3.keypom.testnet" : "v1-3.keypom.near";

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
	console.log('nearConnection: ', nearConnection)

	// Create an account object for the desired account and check to see if it exists already.
	const account = await nearConnection.account(newAccountId);
	console.log('account: ', account)

	// If the call to check state fails, that means the account does not exist and we're free to
	// proceed. Otherwise, we should exit.
	try {
		await account.state();
		console.log("account already exists. Exiting early.")
		return false;
	} catch(e) {console.log('e: ', e)}
	
	// Create the keypom account object which will be used to create the new account and claim
	// The linkdrop.
	const keypomAccountObject = await nearConnection.account(keypomContractId);
	console.log('keypomAccountObject: ', keypomAccountObject)

	// Set the key that the keypom account object will use to sign the claim transaction
	await keyStore.setKey(network, keypomContractId, KeyPair.fromString(privKey));
	console.log('keyStore: ', keyStore)
	
	// Generate a new keypair based on entropy of a hash of the pin code and the new account ID
	let entropy = createHash('sha256').update(Buffer.from(newAccountId.toString() + pinCode.toString())).digest('hex');
	console.log('entropy: ', entropy)
	
	let { seedPhrase, secretKey, publicKey } = await generateSeedPhrase(entropy);
	let newAccountPubKey = publicKey;
	
	console.log('secretKey: ', secretKey);

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
		return false;
	}

	// Generate the auto import link for the new account
	const walletAutoImportLink = `https://wallet.${network}.near.org/auto-import-secret-key#${newAccountId}/${secretKey}`
	console.log('walletAutoImportLink: ', walletAutoImportLink)
	return walletAutoImportLink;
}

createAccountAndClaim("4xUJmuPKEYAHm4748EAYJ8NJoFPAJJJadXy9D6bhuXYVvQXQt9AbdV94mfvkaQC6HwvnpvCox4xBNvztjMmpkX5u", "asdkajsdlaksjdlasdjkddasdasdlkjlasdadfoasd.testnet", "0123");