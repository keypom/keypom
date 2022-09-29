const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');
  
let KEYPOM_CONTRACT = process.env.CONTRACT_NAME;
let FUNDING_ACCOUNT_ID = process.env.FUNDING_ACCOUNT_ID;
let LINKDROP_NEAR_AMOUNT = process.env.LINKDROP_NEAR_AMOUNT;
let NFT_CONTRACT_ID = process.env.NFT_CONTRACT_ID;
let TOKEN_ID = process.env.TOKEN_ID;

let OFFSET = 1;
let DROP_FEE = 1;
let KEY_FEE = 0.005;
let NUM_KEYS = 1;

let NETWORK_ID = "testnet";
let near;
let keyStore;

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

	if(!KEYPOM_CONTRACT) {
		const dev_account = await readFile(`neardev/dev-account`);
		KEYPOM_CONTRACT = dev_account.toString();
	}

	console.log('KEYPOM_CONTRACT: ', KEYPOM_CONTRACT);
	console.log('FUNDING_ACCOUNT_ID: ', FUNDING_ACCOUNT_ID);
	console.log('LINKDROP_NEAR_AMOUNT: ', LINKDROP_NEAR_AMOUNT);

	if(!FUNDING_ACCOUNT_ID || !LINKDROP_NEAR_AMOUNT || !NFT_CONTRACT_ID) {
		throw "must specify funding account and linkdrop near amount and nft contract ID";
	}

	const contractAccount = await near.account(KEYPOM_CONTRACT);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	console.log(`initializing contract for account ${KEYPOM_CONTRACT}`);
	try {
		await contractAccount.functionCall(
			KEYPOM_CONTRACT, 
			'new', 
			{
				root_account: "testnet",
				owner_id: KEYPOM_CONTRACT
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	let keyPairs = [];
	let pubKeys = [];
	let viewData = {};

	console.log("BATCH Creating keypairs");
	for(var i = 0; i < NUM_KEYS; i++) {
		console.log('i: ', i);
		let keyPair = await KeyPair.fromRandom('ed25519'); 
		keyPairs.push(keyPair);   
		pubKeys.push(keyPair.publicKey.toString());   
	}
	console.log("Finished.");

	const dropId = await fundingAccount.viewFunction(
		KEYPOM_CONTRACT, 
		'get_next_drop_id',
	);

	try {
		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'add_to_balance', 
			{},
			"300000000000000", 
			parseNearAmount(
				((parseFloat(LINKDROP_NEAR_AMOUNT) + KEY_FEE + OFFSET) * pubKeys.length * 1 + DROP_FEE).toString()
			)
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	try {
		let nft_data = {};
		nft_data["contract_id"] = NFT_CONTRACT_ID;
		nft_data["sender_id"] = FUNDING_ACCOUNT_ID;

		await fundingAccount.functionCall(
			KEYPOM_CONTRACT, 
			'create_drop', 
			{
				public_keys: pubKeys,
				deposit_per_use: parseNearAmount(LINKDROP_NEAR_AMOUNT),
				nft_data
			}, 
			"300000000000000"
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	try {
		console.log(`transferring NFT to linkdrop proxy contract with nft_transfer_call`);
		await fundingAccount.functionCall(
			NFT_CONTRACT_ID, 
			'nft_transfer_call', 
			{
				token_id: TOKEN_ID,
				receiver_id: KEYPOM_CONTRACT,
				msg: dropId.toString(),
			}, 
			"300000000000000", 
			'1'
		);
	} catch(e) {
		console.log('error sending FTs: ', e);
	}

	let curPks = {};
	for(var i = 0; i < keyPairs.length; i++) {
		curPks[keyPairs[i].publicKey.toString()] = `https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`;
		console.log(`https://testnet.mynearwallet.com/linkdrop/${KEYPOM_CONTRACT}/${keyPairs[i].secretKey}`);
		console.log("Pub Key: ", keyPairs[i].publicKey.toString());
	}

	await writeFile(path.resolve(__dirname, `pks.json`), JSON.stringify(curPks));
}


start();