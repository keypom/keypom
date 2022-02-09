const fs = require("fs");
const BN = require('bn.js');
const fetch = require('node-fetch');
const nearAPI = require('near-api-js');
const { KeyPair, Account, Contract, utils: { format: { parseNearAmount, formatNearAmount } } } = nearAPI;
const { near, credentials, connection, keyStore, contract, contractAccount } = require('./near-utils');
const getConfig = require('./config');
const {
	networkId, contractName, contractMethods, gas,
	NEW_ACCOUNT_AMOUNT, 
	DEFAULT_NEW_CONTRACT_AMOUNT,
} = getConfig('testnet');

const format = (amount) => {
	const res = formatNearAmount(amount, 8)
	if (res.indexOf('-') > -1) {
		return '-' + res.replace('-', '')
	}
	return res
}

const TEST_HOST = 'http://localhost:3000';
/// exports
async function initContract() {
	/// try to call new on contract, swallow e if already initialized
	try {
		const newArgs = {
			linkdrop_contract: contractId
		};
		await contract.new(newArgs);
	} catch (e) {
		if (!/initialized/.test(e.toString())) {
			throw e;
		}
	}
	return { contract, contractName };
}

const initAccount = async(accountId, secret) => {
	account = new nearAPI.Account(connection, accountId);
	const newKeyPair = KeyPair.fromString(secret);
	keyStore.setKey(networkId, accountId, newKeyPair);
	return account;
};

const createOrInitAccount = async(accountId, secret, amount = DEFAULT_NEW_CONTRACT_AMOUNT) => {
	let account;
	try {
		account = await createAccount(accountId, amount, secret);
	} catch (e) {
		if (!/because it already exists/.test(e.toString())) {
			throw e;
		}
		account = initAccount(accountId, secret);
	}
	return account;
};

const getAccount = async (accountId, fundingAmount = NEW_ACCOUNT_AMOUNT, secret) => {
	const account = new nearAPI.Account(connection, accountId);
	try {
		let secret;
		try {
			secret = JSON.parse(fs.readFileSync(process.env.HOME + `/.near-credentials/${networkId}/${accountId}.json`, 'utf-8')).private_key;
		} catch(e) {
			if (!/no such file|does not exist/.test(e.toString())) {
				throw e;
			}
			secret = fs.readFileSync(`./neardev/${accountId}`, 'utf-8');
		}
		const newKeyPair = KeyPair.fromString(secret);
		keyStore.setKey(networkId, accountId, newKeyPair);
		await account.state();
		return account;
	} catch(e) {
		if (!/no such file|does not exist/.test(e.toString())) {
			throw e;
		}
	}
	return await createAccount(accountId, fundingAmount, secret);
};


async function getContract(account) {
	return new Contract(account || contractAccount, contractName, {
		...contractMethods,
		signer: account || undefined
	});
}


const createAccessKeyAccount = (key) => {
	connection.signer.keyStore.setKey(networkId, contractName, key);
	return new Account(connection, contractName);
};

const postSignedJson = async ({ account, contractName, url, data = {} }) => {
	return await fetch(url, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({
			...data,
			accountId: account.accountId,
			contractName,
			...(await getSignature(account))
		})
	}).then((res) => {
		// console.log(res)
		return res.json();
	});
};

const postJson = async ({ url, data = {} }) => {
	return await fetch(url, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ ...data })
	}).then((res) => {
		console.log(res);
		return res.json();
	});
};

function generateUniqueSubAccount() {
	return `t${Date.now()}.${contractName}`;
}

/// internal
const createAccount = async (accountId, fundingAmount = NEW_ACCOUNT_AMOUNT, secret) => {
	const newKeyPair = secret ? KeyPair.fromString(secret) : KeyPair.fromRandom('ed25519');
	fs.writeFileSync(`./neardev/${accountId}` , newKeyPair.toString(), 'utf-8');
	await contractAccount.createAccount(accountId, newKeyPair.publicKey, fundingAmount);
	keyStore.setKey(networkId, accountId, newKeyPair);
	return new nearAPI.Account(connection, accountId);
};

const getSignature = async (account) => {
	const { accountId } = account;
	const block = await account.connection.provider.block({ finality: 'final' });
	const blockNumber = block.header.height.toString();
	const signer = account.inMemorySigner || account.connection.signer;
	const signed = await signer.signMessage(Buffer.from(blockNumber), accountId, networkId);
	const blockNumberSignature = Buffer.from(signed.signature).toString('base64');
	return { blockNumber, blockNumberSignature };
};

const loadCredentials = (accountId) => {
	const credPath = `./neardev/${networkId}/${accountId}.json`;
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
				`${process.env.HOME}/.near-credentials/${networkId}/${accountId}.json`
			)
		);
	}

	return credentials;
};

/// debugging

const getAccountBalance = (accountId) => (new nearAPI.Account(connection, accountId)).getAccountBalance();
const getAccountState = (accountId) => (new nearAPI.Account(connection, accountId)).state();
const totalDiff = (balanceBefore, balanceAfter) => format(new BN(balanceAfter.total).sub(new BN(balanceBefore.total)).toString());
const availableDiff = (balanceBefore, balanceAfter) => format(new BN(balanceAfter.available).sub(new BN(balanceBefore.available)).toString());
const stateCost = (balanceBefore, balanceAfter) => format(new BN(balanceAfter.stateStaked).sub(new BN(balanceBefore.stateStaked)).toString());
const bytesUsed = (stateBefore, stateAfter) => parseInt(stateAfter.storage_usage, 10) - parseInt(stateBefore.storage_usage);

/// analyzing

let data = {};
const recordStart = async (accountId) => {
	data[accountId] = {
		balance: await getAccountBalance(accountId),
		state: await getAccountState(accountId),
	};
};

const recordStop = async (accountId) => {
	const before = data[accountId];
	const after = {
		balance: await getAccountBalance(accountId),
		state: await getAccountState(accountId),
	};

	console.log(format(before.balance.total), format(after.balance.total))

	console.log(
		'\n', 'Analysis:', '\n',
		'Total diff:', totalDiff(before.balance, after.balance), '\n',
		'Avail diff:', availableDiff(before.balance, after.balance), '\n',
		'State used:', stateCost(before.balance, after.balance), '\n',
		'Bytes used:', bytesUsed(before.state, after.state), '\n',
	);
};

module.exports = { 
	recordStart,
	recordStop,
	TEST_HOST,
	near,
	gas,
	connection,
	credentials,
	keyStore,
	getContract,
	getAccountBalance,
	contract,
	contractName,
	networkId,
	contractId: contractName,
	contractMethods,
	contractAccount,
	initAccount,
	createOrInitAccount,
	createAccessKeyAccount,
	initContract, getAccount, postSignedJson, postJson,
	loadCredentials,
};

