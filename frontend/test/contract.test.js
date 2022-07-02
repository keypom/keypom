const test = require('ava');
const {
	getAccount, init,
	recordStart, recordStop,
} = require('./test-utils');
const getConfig = require("../utils/config");
const {
	contractId,
	gas,
	attachedDeposit,
} = getConfig();

// test.beforeEach((t) => {
// });

let contractAccount, event_name, aliceId, bobId, alice, bob;

test('contract is deployed', async (t) => {
	contractAccount = await init();

	t.is(contractId, contractAccount.accountId);
});

test('users initialized', async (t) => {
	aliceId = 'alice.' + contractId;
	bobId = 'bob.' + contractId;
	alice = await getAccount(aliceId);
	bob = await getAccount(bobId);

	t.true(true);
});

test('create an event', async (t) => {
	event_name = 'event-' + Date.now();

	const res = await contractAccount.functionCall({
		contractId,
		methodName: 'create_event',
		args: {
			event_name,
		},
		gas,
		attachedDeposit,
	});

	t.is(res?.status?.SuccessValue, '');
});

test('get events', async (t) => {
	const res = await contractAccount.viewFunction(
		contractId,
		'get_events',
		{}
	);

	// console.log(res)

	t.true(res.length >= 1);
});

test('create a connection', async (t) => {

	await recordStart(contractId);
	
	const res = await alice.functionCall({
		contractId,
		methodName: 'create_connection',
		args: {
			event_name,
			new_connection_id: bobId,
		},
		gas,
		attachedDeposit,
	});

	await recordStop(contractId);

	t.is(res?.status?.SuccessValue, '');
});

test('create another connection', async (t) => {

	const carolId = 'car.' + contractId;

	await recordStart(contractId);

	const res = await alice.functionCall({
		contractId,
		methodName: 'create_connection',
		args: {
			event_name,
			new_connection_id: carolId,
		},
		gas,
		attachedDeposit,
	});
	
	await recordStop(contractId);

	t.is(res?.status?.SuccessValue, '');
});

test('get connections', async (t) => {
	const res = await alice.viewFunction(
		contractId,
		'get_connections',
		{
			event_name,
			network_owner_id: aliceId,
		}
	);

	console.log(res);

	t.true(res.length >= 1);
});