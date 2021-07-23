const assert = require('assert');
const { KeyPair } = require('near-api-js');
const { parseNearAmount } = require('near-api-js/lib/utils/format');
const testUtils = require('./test-utils');

const {
	near,
	networkId,
	contractId,
	contractAccount,
} = testUtils;

// 50 Tgas is enough
const gas = '50000000000000';

describe('Linkdrop Wrapper', function () {
	this.timeout(20000);

	// linkdrop keypairs
	const keyPair1 = KeyPair.fromRandom('ed25519')
	const keyPair2 = KeyPair.fromRandom('ed25519')
	const public_key1 = keyPair1.publicKey.toString()
	const public_key2 = keyPair2.publicKey.toString()
	// the new account's keypair
	const keyPairNewAccount = KeyPair.fromRandom('ed25519')
	const new_public_key = keyPairNewAccount.publicKey.toString()

	it('contract deployed', async function() {
		const state = await contractAccount.state()
		try {
			await contractAccount.functionCall({
				contractId,
				methodName: 'new',
				args: {
					linkdrop_contract: 'testnet'
				},
				gas
			})
		} catch (e) {
			if (!/contract has already been initialized/.test(e.toString())) {
				console.warn(e)
			}
		}

		assert.notStrictEqual(state.code_hash, '11111111111111111111111111111111');
	});

	it('creation of linkdrop and wallet link for testing', async function() {
		await contractAccount.functionCall({
			contractId,
			methodName: 'send',
			args: {
				public_key: public_key1
			},
			gas,
			// could be 0.02 N wallet needs to reduce gas from 100 Tgas to 50 Tgas
			attachedDeposit: parseNearAmount('0.03')
		})

		console.log(`https://wallet.testnet.near.org/linkdrop/${contractId}/${keyPair1.secretKey}?redirectUrl=https://example.com`)

		return true
	});

	it('creation of linkdrop', async function() {
		const res = await contractAccount.functionCall({
			contractId,
			methodName: 'send',
			args: {
				public_key: public_key2
			},
			gas,
			attachedDeposit: parseNearAmount('0.02')
		})

		assert.strictEqual(res.status.SuccessValue, '');
	});

	it('creation of account', async function() {
		// WARNING tests after this with contractAccount will fail - signing key lost
		// set key for contractAccount to linkdrop keyPair
		near.connection.signer.keyStore.setKey(networkId, contractId, keyPair2);

		const res = await contractAccount.functionCall({
			contractId,
			methodName: 'create_account_and_claim',
			args: {
				new_account_id: 'test-linkdrop-wrapper-' + Date.now().toString(),
				new_public_key,
			},
			gas,
		})

		// console.log(res)
		// true
		assert.strictEqual(res.status.SuccessValue, 'dHJ1ZQ==');
	});

	// WARNING tests after this with contractAccount will fail - signing key lost

	

})
