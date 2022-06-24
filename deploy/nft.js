const { connect, KeyPair, keyStores, utils } = require("near-api-js");
const { parseNearAmount, formatNearAmount } = require("near-api-js/lib/utils/format");
const path = require("path");
const homedir = require("os").homedir();
  
let LINKDROP_PROXY_CONTRACT_ID = process.env.LINKDROP_PROXY_CONTRACT_ID;
let FUNDING_ACCOUNT_ID = process.env.FUNDING_ACCOUNT_ID;
let LINKDROP_NEAR_AMOUNT = process.env.LINKDROP_NEAR_AMOUNT;
let SEND_MULTIPLE = process.env.SEND_MULTIPLE;

let OFFSET = 2;

let NETWORK_ID = "testnet";
let near;
let config;
let keyStore;

/*
	Hard coding NFT contract and metadata. Change this if you want.
*/
let NFT_CONTRACT_ID = "nft.examples.testnet";
const METADATA = {
	"title": "Linkdropped Go Team NFT",
	"description": "Testing Linkdrop NFT Go Team Token",
	"media": "https://bafybeiftczwrtyr3k7a2k4vutd3amkwsmaqyhrdzlhvpt33dyjivufqusq.ipfs.dweb.link/goteam-gif.gif",
	"media_hash": null,
	"copies": 10000,
	"issued_at": null,
	"expires_at": null,
	"starts_at": null,
	"updated_at": null,
	"extra": null,
	"reference": null,
	"reference_hash": null
};

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
				linkdrop_contract: "testnet",
			}, 
			"300000000000000", 
		);
	} catch(e) {
		console.log('error initializing contract: ', e);
	}
	
	let keyPairs = [];
	let pubKeys = [];
	let nft_data = [];

	if(SEND_MULTIPLE != "false") {
		console.log("BATCH Creating keypairs");
		for(var i = 0; i < 5; i++) {
			console.log('i: ', i);
			let keyPair = await KeyPair.fromRandom('ed25519'); 
			keyPairs.push(keyPair);   
			pubKeys.push(keyPair.publicKey.toString());  
			
			nft_data.push({
				nft_sender: FUNDING_ACCOUNT_ID,
				nft_contract: NFT_CONTRACT_ID,
				nft_token_id: keyPair.publicKey.toString()
			});
		}
		console.log("Finished.");
	} else {
		let keyPair = await KeyPair.fromRandom('ed25519'); 
		keyPairs.push(keyPair);   
		pubKeys.push(keyPair.publicKey.toString());  
		
		nft_data.push({
			nft_sender: FUNDING_ACCOUNT_ID,
			nft_contract: NFT_CONTRACT_ID,
			nft_token_id: keyPair.publicKey.toString()
		});
	}

	try {
		if(SEND_MULTIPLE != "false") {
			await fundingAccount.functionCall(
				LINKDROP_PROXY_CONTRACT_ID, 
				'send_multiple', 
				{
					public_keys: pubKeys,
					balance: parseNearAmount(LINKDROP_NEAR_AMOUNT),
					nft_data
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
					nft_data: nft_data[0]
				}, 
				"300000000000000", 
				parseNearAmount((parseFloat(LINKDROP_NEAR_AMOUNT) + OFFSET).toString())
			);
		}
		
	} catch(e) {
		console.log('error initializing contract: ', e);
	}
	
	try {
		for(var i = 0; i < pubKeys.length; i++) {
			console.log(`minting NFT with token ID ${nft_data[i].nft_token_id} on contract ${NFT_CONTRACT_ID} with receiver: ${FUNDING_ACCOUNT_ID}`);
			
			await fundingAccount.functionCall(
				NFT_CONTRACT_ID, 
				'nft_mint', 
				{
					token_id: nft_data[i].nft_token_id,
					receiver_id: FUNDING_ACCOUNT_ID,
					token_metadata: METADATA,
				}, 
				"300000000000000", 
				parseNearAmount('1')
			);

			console.log(`transferring NFT to linkdrop proxy contract with nft_transfer_call`);
			await fundingAccount.functionCall(
				NFT_CONTRACT_ID, 
				'nft_transfer_call', 
				{
					token_id: nft_data[i].nft_token_id,
					receiver_id: LINKDROP_PROXY_CONTRACT_ID,
					msg: pubKeys[i],
				}, 
				"300000000000000", 
				'1'
			);
		}
	} catch(e) {
		console.log('error minting and sending NFTs: ', e);
	}
    
	for(var i = 0; i < keyPairs.length; i++) {
		console.log(`https://wallet.testnet.near.org/linkdrop/${LINKDROP_PROXY_CONTRACT_ID}/${keyPairs[i].secretKey}`);
		console.log("Pub Key: ", keyPairs[i].publicKey.toString());
	}
}

start();