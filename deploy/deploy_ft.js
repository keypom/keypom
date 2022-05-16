const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();

let LINKDROP_PROXY_CONTRACT_ID = "dev-1652477128472-86692119156549";
let FUNDING_ACCOUNT_ID = "benjiman.testnet";
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
	const contractId = LINKDROP_PROXY_CONTRACT_ID;
	await initiateNear();

	const contractAccount = await near.account(contractId);

	const sendingAccount = await near.account(FUNDING_ACCOUNT_ID);

	let state = await contractAccount.state(); 
	console.log('state before initialization: ', state);

	console.log("initializing contract");
	try {
		await contractAccount.functionCall(
			contractId, 
			'new', 
			{
				linkdrop_contract: "testnet",
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}
	
	let keyPair = await KeyPair.fromRandom('ed25519'); 
	let pubKey = keyPair.publicKey.toString();
	console.log('pubKey: ', pubKey);

	state = await contractAccount.state(); 
	console.log('state after initialization: ', state);

	console.log("sending funds as FUNDING_ACCOUNT_ID");
	try {
		await sendingAccount.functionCall(
			contractId, 
			'send', 
			{
				public_key: pubKey,
				balance: parseNearAmount("0.01"),
				ft_contract_id: 'ft.examples.benjiman.testnet', 
			}, 
			"300000000000000", 
			parseNearAmount('1')
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}

	state = await contractAccount.state(); 
	console.log('state after send: ', state);
    
	console.log("Paying storage and Sending FTs to public key");
	try {
		const tokenId = Date.now().toString();
		await sendingAccount.functionCall(
			'ft.examples.benjiman.testnet', 
			'storage_deposit', 
			{
				account_id: contractId,
			}, 
			"300000000000000", 
			parseNearAmount('1')
		);

		await sendingAccount.functionCall(
			'ft.examples.benjiman.testnet', 
			'ft_transfer_call', 
			{
				receiver_id: contractId,
				amount: "25",
				msg: pubKey,
			}, 
			"300000000000000", 
			'1'
		);
	} catch(e) {
		console.log('error minting and sending NFTs: ', e);
	}

	state = await contractAccount.state(); 
	console.log('state after mint: ', state);

	console.log(`https://wallet.testnet.near.org/linkdrop/${contractId}/${keyPair.secretKey}`);
}

start();