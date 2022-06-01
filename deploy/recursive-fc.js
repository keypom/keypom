const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
  
let LINKDROP_PROXY_CONTRACT_ID = process.env.LINKDROP_PROXY_CONTRACT_ID;
let FUNDING_ACCOUNT_ID = process.env.FUNDING_ACCOUNT_ID;
let LINKDROP_NEAR_AMOUNT = process.env.LINKDROP_NEAR_AMOUNT;

let OFFSET = 0.0;
let STORAGE = 0.038;

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

	if(!LINKDROP_PROXY_CONTRACT_ID || !FUNDING_ACCOUNT_ID || !LINKDROP_NEAR_AMOUNT) {
		throw "must specify proxy contract ID, funding account ID and linkdrop $NEAR amount";
	}

	const contractAccount = await near.account(LINKDROP_PROXY_CONTRACT_ID);
	const fundingAccount = await near.account(FUNDING_ACCOUNT_ID);

	console.log(`initializing contract for account ${LINKDROP_PROXY_CONTRACT_ID}`);
	try {
		await contractAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'new', 
			{
				linkdrop_contract: "testnet",
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	let keyPairs = [];
	let pubKeys = [];
	console.log("Creating keypairs");
	for(var i = 0; i < 5; i++) {
		let keyPair = await KeyPair.fromRandom('ed25519'); 
		keyPairs.push(keyPair);   
		pubKeys.push(keyPair.publicKey.toString());   
	}
	console.log("Finished.");

	console.log(`sending ${LINKDROP_NEAR_AMOUNT} $NEAR as ${FUNDING_ACCOUNT_ID}`);
	try {
		let fc_data_base = {};
		let argsBase = btoa(JSON.stringify({
			public_key:  keyPairs[0].publicKey.toString(),
			balance: parseNearAmount(LINKDROP_NEAR_AMOUNT),
		}));
		fc_data_base["receiver"] = LINKDROP_PROXY_CONTRACT_ID;
		fc_data_base["method"] = "send";
		fc_data_base["args"] = argsBase;
		fc_data_base["deposit"] = parseNearAmount((parseFloat(LINKDROP_NEAR_AMOUNT) + OFFSET + STORAGE).toString());
		console.log("Base case deposit: ", (parseFloat(LINKDROP_NEAR_AMOUNT) + OFFSET + STORAGE).toString());
		
		let argsArray = [];
		argsArray.push(fc_data_base);
		console.log('argsArray: ', argsArray);

		let fc_data_final = {};
		for(var i = 1; i < keyPairs.length-1; i++) {
			let args = btoa(JSON.stringify({
				public_key:  keyPairs[i].publicKey.toString(),
				balance: parseNearAmount(LINKDROP_NEAR_AMOUNT),
				fc_data: argsArray[i - 1],
			}));
            
			fc_data_final["receiver"] = LINKDROP_PROXY_CONTRACT_ID;
			fc_data_final["method"] = "send";
			fc_data_final["args"] = args;
			fc_data_final["deposit"] = parseNearAmount(((parseFloat(LINKDROP_NEAR_AMOUNT) + OFFSET + STORAGE) * (i+1)).toString());
			console.log("deposit for iter: ", i, " : ", ((parseFloat(LINKDROP_NEAR_AMOUNT) + OFFSET + STORAGE)) * (i+1).toString());
            
			//console.log('fc_data_final: ', fc_data_final);
			console.log('args Length: ', args.length);

			argsArray.push(fc_data_final);
		}
        
		console.log('fc_data_final: ', fc_data_final);
		
		await fundingAccount.functionCall(
			LINKDROP_PROXY_CONTRACT_ID, 
			'send', 
			{
				public_key: keyPairs[keyPairs.length - 1].publicKey.toString(),
				balance: parseNearAmount(LINKDROP_NEAR_AMOUNT),
				fc_data: fc_data_final,
			}, 
			"300000000000000", 
			parseNearAmount(((parseFloat(LINKDROP_NEAR_AMOUNT) + OFFSET + STORAGE) * (keyPairs.length)).toString())
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}
    
	for(var i = 0; i < keyPairs.length; i++) {
		console.log(`https://wallet.testnet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`);
		console.log("Pub Key: ", keyPairs[i].publicKey.toString());
	}
}


start();