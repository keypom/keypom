const fs = require("fs");
const nearAPI = require("near-api-js");
const getConfig = require("../test/config");

// for testnet
const network = 'testnet'
const { nodeUrl, networkId, GAS } = getConfig(network);
const contractId = 'linkdrop-wrapper.' + network

const {
	keyStores: { InMemoryKeyStore },
	Near,
	Account,
	KeyPair,
	transactions: { deployContract, functionCall },
} = nearAPI;

const credPath = `./neardev/${networkId}/${contractId}.json`;
console.log(
	"Loading Credentials:\n",
	credPath
);

let credentials;
try {
	credentials = JSON.parse(
		fs.readFileSync(
			credPath
		)
	);
} catch(e) {
	console.warn('credentials not in /neardev');
	/// attempt to load backup creds from local machine
	credentials = JSON.parse(
		fs.readFileSync(
			`${process.env.HOME}/.near-credentials/${networkId}/${contractId}.json`
		)
	);
}

const keyStore = new InMemoryKeyStore();
keyStore.setKey(
	networkId,
	contractId,
	KeyPair.fromString(credentials.private_key)
);
const near = new Near({
	networkId,
	nodeUrl,
	deps: { keyStore },
});
const { connection } = near;
const contractAccount = new Account(connection, contractId);

async function init () {
	const contractBytes = fs.readFileSync('./out/main.wasm');
	console.log('\n\n deploying contractBytes:', contractBytes.length, '\n\n');

	const actions = [
		deployContract(contractBytes),
	];
	const state = await contractAccount.state()
	if (state.code_hash === '11111111111111111111111111111111') {
		actions.push(functionCall('new', { linkdrop_contract: network }, GAS))
	}
	await contractAccount.signAndSendTransaction({ receiverId: contractId, actions });
}
init()

