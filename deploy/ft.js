const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
  
let LINKDROP_PROXY_CONTRACT_ID = process.env.LINKDROP_PROXY_CONTRACT_ID;
let FUNDING_ACCOUNT_ID = process.env.FUNDING_ACCOUNT_ID;
let LINKDROP_NEAR_AMOUNT = process.env.LINKDROP_NEAR_AMOUNT;
let FT_CONTRACT_ID = process.env.FT_CONTRACT_ID;
let SEND_MULTIPLE = process.env.SEND_MULTIPLE;

let NUM_KEYS_IF_SEND_MULTIPLE = 130;
let OFFSET = 2;
let NETWORK_ID = "testnet";
let near;
let config;
let keyStore;

// set up near
const initiateNear = async () => {
	const CREDENTIALS_DIR = ".near-credentials";

	const credentialsPath = (await path).join(homedir, CREDENTIALS_DIR);
	(await path).join;
	keyStore = new keyStores.UnencryptedFileSystemKeyStore(credentialsPath);

	config = {
		networkId: NETWORK_ID,
		keyStore,
		nodeUrl: "https://rpc.testnet.near.org",
		walletUrl: "https://wallet.testnet.near.org",
		helperUrl: "https://helper.testnet.near.org",
		explorerUrl: "https://explorer.testnet.near.org",
	};

	near = await connect(config);
};

async function start() {
	//deployed linkdrop proxy contract
	await initiateNear();

	if(!LINKDROP_PROXY_CONTRACT_ID || !FUNDING_ACCOUNT_ID || !LINKDROP_NEAR_AMOUNT || !SEND_MULTIPLE) {
		throw "must specify proxy contract ID, funding account ID, linkdrop $NEAR amount and whether to send multiple";
	}

	const contractAccount = await near.account(LINKDROP_PROXY_CONTRACT_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	console.log(`initializing contract for account ${LINKDROP_PROXY_CONTRACT_ID}`);
	try {
		await contractAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'new', 
			{
				linkdrop_contract: "testnet"
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}
	
	let keyPairs = [];
	let pubKeys = [];

	if(SEND_MULTIPLE != "false") {
		console.log("BATCH Creating keypairs");
		for(var i = 0; i < NUM_KEYS_IF_SEND_MULTIPLE; i++) {
			console.log('i: ', i);
			let keyPair = await KeyPair.fromRandom('ed25519'); 
			keyPairs.push(keyPair);   
			pubKeys.push(keyPair.publicKey.toString());  
		}
		console.log("Finished.");
	} else {
		let keyPair = await KeyPair.fromRandom('ed25519'); 
		keyPairs.push(keyPair);   
		pubKeys.push(keyPair.publicKey.toString());  
	}

	console.log(`sending ${LINKDROP_NEAR_AMOUNT} $NEAR as ${FUNDING_ACCOUNT_ID}`);
	try {
		let ft_data = {};
		ft_data["ft_contract"] = FT_CONTRACT_ID;
		ft_data["ft_sender"] = FUNDING_ACCOUNT_ID;
		ft_data["ft_balance"] = "25";

		if(SEND_MULTIPLE != "false") {
			await fundingAccount.functionCall(
				LINKDROP_PROXY_CONTRACT_ID, 
				'send_multiple', 
				{
					public_keys: pubKeys,
					balance: parseNearAmount(LINKDROP_NEAR_AMOUNT),
					ft_data
				}, 
				"300000000000000", 
				parseNearAmount(((parseFloat(LINKDROP_NEAR_AMOUNT) + OFFSET) * pubKeys.length).toString())
			);
		} else {
			console.log("Sending one linkdrop");
			await fundingAccount.functionCall(
				LINKDROP_PROXY_CONTRACT_ID, 
				'send', 
				{
					public_key: pubKeys[0],
					balance: parseNearAmount(LINKDROP_NEAR_AMOUNT),
					ft_data
				}, 
				"300000000000000", 
				parseNearAmount((parseFloat(LINKDROP_NEAR_AMOUNT) + OFFSET).toString())
			);
		}
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	try {
		console.log(`Paying for FT storage on contract: ${FT_CONTRACT_ID} for the proxy contract ID`);
		await fundingAccount.functionCall(
			FT_CONTRACT_ID, 
			'storage_deposit', 
			{
				account_id: LINKDROP_PROXY_CONTRACT_ID,
			}, 
			"300000000000000", 
			parseNearAmount('1')
		);
		for(var i = 0; i < pubKeys.length; i++) {
			console.log(`Transferring 25 FTs from ${FUNDING_ACCOUNT_ID} to ${LINKDROP_PROXY_CONTRACT_ID}`);
			await fundingAccount.functionCall(
				FT_CONTRACT_ID, 
				'ft_transfer_call', 
				{
					receiver_id: LINKDROP_PROXY_CONTRACT_ID,
					amount: "25",
					msg: pubKeys[i],
				}, 
				"300000000000000", 
				'1'
			);
		}
	} catch(e) {
		console.log('error sending FTs: ', e);
	}
	
	for(var i = 0; i < keyPairs.length; i++) {
		console.log(`https://wallet.testnet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`);
		console.log("Pub Key: ", keyPairs[i].publicKey.toString());
	}
}

start();