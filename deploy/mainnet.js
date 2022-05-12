const fs = require("fs");
const nearAPI = require("near-api-js");
const getConfig = require("../test/config");

// for testnet
const network = 'mainnet'
let { nodeUrl, networkId, GAS } = getConfig(network);
GAS = "200000000000000"
const contractId = 'ldproxy.near'

const {
	keyStores: { InMemoryKeyStore },
	Near,
	Account,
	KeyPair,
	transactions: { deployContract, functionCall },
} = nearAPI;


const credPath = `${process.env.HOME}/.near-credentials/${networkId}/${contractId}.json`
console.log(
	"Loading Credentials:\n",
	credPath
);
let credentials = JSON.parse(
	fs.readFileSync(
		credPath
	)
);

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

const linkdrop_contract = 'near'

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

	if (linkdrop_contract) {
		actions.push(functionCall('set_contract', { linkdrop_contract }, GAS))
	}

	await contractAccount.signAndSendTransaction({ receiverId: contractId, actions });
}
init()

