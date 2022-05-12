const assert = require('assert');
const { KeyPair, Account } = require('near-api-js');
const { parseNearAmount } = require('near-api-js/lib/utils/format');
const testUtils = require('./test-utils');

let {
	near,
	networkId,
	contractId,
	contractAccount,
	recordStart,
	recordStop,
	getAccount,
} = testUtils;

let linkdropAccount = contractAccount;
/// contractAccount is the devAccount - testing against deployed contract on testnet
const useDeployedLinkdrop = false;
if (useDeployedLinkdrop) {
	contractId = 'linkdrop-wrapper.testnet';
	linkdropAccount = new Account(near.connection, contractId);
}

// 85 Tgas is enough with callback check
const gas = '85000000000000';
const gasMultiple = '200000000000000';
const attachedDeposit = parseNearAmount('0.03')
const attachedDepositDouble = parseNearAmount('0.06')

describe('Linkdrop Proxy', function () {
	this.timeout(60000);

	const aliceId = 'alice-test.' + contractId

	// linkdrop keypairs
	const keyPair1 = KeyPair.fromRandom('ed25519');
	const keyPair2 = KeyPair.fromRandom('ed25519');
	const public_key1 = keyPair1.publicKey.toString();
	const public_key2 = keyPair2.publicKey.toString();
	// the new account's keypair
	const keyPairNewAccount = KeyPair.fromRandom('ed25519');
	const new_public_key = keyPairNewAccount.publicKey.toString();

	it('accounts and contract deployed', async function() {

		alice = await getAccount(aliceId);
		// console.log(alice)

		const state = await linkdropAccount.state();
		if (state.code_hash.indexOf('111111') === 0) {
			return assert(true)
		}
		try {
			await contractAccount.functionCall({
				contractId,
				methodName: 'new',
				args: {
					linkdrop_contract: 'testnet',
				},
				gas
			});
		} catch (e) {
			if (!/contract has already been initialized/.test(e.toString())) {
				console.warn(e);
			}
		}
		
		assert.notStrictEqual(state.code_hash, '11111111111111111111111111111111');
	});

	// it('creation of linkdrop and wallet link for testing', async function() {

	// 	await alice.functionCall({
	// 		contractId,
	// 		methodName: 'send',
	// 		args: {
	// 			public_key: public_key1
	// 		},
	// 		gas,
	// 		// could be 0.02 N wallet needs to reduce gas from 100 Tgas to 50 Tgas
	// 		attachedDeposit
	// 	});

	// 	console.log(`https://wallet.testnet.near.org/linkdrop/${contractId}/${keyPair1.secretKey}?redirectUrl=https://example.com`);

	// 	return true;
	// });

	/// keyPair1

	it('creation of linkdrops', async function() {

		const EXTRA = 0
		const extraKeys = []
		for (let i = 0; i < EXTRA; i++) {
			extraKeys.push(KeyPair.fromRandom('ed25519').publicKey.toString())
		}

		await recordStart(contractId)

		const res = await alice.functionCall({
			contractId,
			methodName: 'send_multiple',
			args: {
				public_keys: [public_key1, public_key2, ...extraKeys],
				balance: 0,
			},
			gas: gasMultiple,
			attachedDeposit: parseNearAmount((0.03 * (EXTRA+2)).toString())
		});

		console.log(`https://wallet.testnet.near.org/linkdrop/${contractId}/${keyPair1.secretKey}?redirectUrl=https://example.com`);
		console.log(`https://wallet.testnet.near.org/linkdrop/${contractId}/${keyPair2.secretKey}?redirectUrl=https://example.com`);

		assert.strictEqual(res.status.SuccessValue, '');
	});

	// it('creation of account', async function() {
	// 	// WARNING tests after this with contractAccount will fail - signing key lost
	// 	// set key for contractAccount to linkdrop keyPair
	// 	near.connection.signer.keyStore.setKey(networkId, contractId, keyPair1);
	// 	const new_account_id = 'linkdrop-wrapper-' + Date.now().toString() + '.testnet';

	// 	const res = await linkdropAccount.functionCall({
	// 		contractId,
	// 		methodName: 'create_account_and_claim',
	// 		args: {
	// 			new_account_id,
	// 			new_public_key,
	// 		},
	// 		gas,
	// 	});

	// 	await recordStop(contractId)

	// 	console.log('created account', new_account_id)

	// 	try {
	// 		await (new Account(near.connection, new_account_id)).state()
	// 		assert(true)
	// 	} catch (e) {
	// 		assert(false)
	// 	}
	// });

	/// keyPair2

	// it('creation of linkdrop', async function() {

	// 	await recordStart(contractId)

	// 	const res = await alice.functionCall({
	// 		contractId,
	// 		methodName: 'send',
	// 		args: {
	// 			public_key: public_key2
	// 		},
	// 		gas,
	// 		attachedDeposit
	// 	});

	// 	assert.strictEqual(res.status.SuccessValue, '');
	// });

	// it('claim of linkdrop', async function() {
	// 	// WARNING tests after this with contractAccount will fail - signing key lost
	// 	// set key for contractAccount to linkdrop keyPair
	// 	near.connection.signer.keyStore.setKey(networkId, contractId, keyPair2);
	// 	const account_id = 'testnet';

	// 	const res = await linkdropAccount.functionCall({
	// 		contractId,
	// 		methodName: 'claim',
	// 		args: {
	// 			account_id,
	// 		},
	// 		gas,
	// 	});

	// 	await recordStop(contractId)

	// 	// console.log(res)

	// 	assert(true)
	// });

	/// testing if promise fails (must edit contract->on_account_created to return false)
	// it('creation of account - FAIL', async function() {
	// 	near.connection.signer.keyStore.setKey(networkId, contractId, keyPair2);
	// 	const new_account_id = 'linkdrop-wrapper-' + Date.now().toString();

	// 	try {
	// 		const res = await linkdropAccount.functionCall({
	// 			contractId,
	// 			methodName: 'create_account_and_claim',
	// 			args: {
	// 				new_account_id,
	// 				new_public_key,
	// 			},
	// 			gas,
	// 		});
	
	// 		console.log(new_account_id);
	// 		console.log(Buffer.from(res.status.SuccessValue, 'base64').toString('utf-8'))
	
	// 		// console.log(res)
	// 		// true
	// 		assert.strictEqual(res.status.SuccessValue, 'dHJ1ZQ==');
	// 	} catch(e) {
	// 		console.log('fail')
	// 		console.log(keyPair2.publicKey.toString())
	// 	}
		
	// });

});
