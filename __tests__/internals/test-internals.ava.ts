import { Worker, NearAccount, NEAR, KeyPairEd25519 } from "near-workspaces";
import anyTest, { TestFn } from "ava";
import { assertBalanceChange, defaultCallOptions, DEFAULT_DEPOSIT, DEFAULT_GAS } from "../utils/utils";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
  }>;

test.beforeEach(async (t) => {
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy the dropzone contract.
    const dropzone = await root.devDeploy(`./out/main.wasm`);

    // Init the contract
    await dropzone.call(dropzone, 'new', {root_account: 'testnet', owner_id: dropzone});

    // Test users
    const ali = await root.createSubAccount('ali');
    const bob = await root.createSubAccount('bob');

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, dropzone, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Initial nonce is 0', async t => {
    const { dropzone } = t.context.accounts;
    const result = await dropzone.view('get_next_drop_id', {});
    t.is(result, 0);
});

test('Changing linkdrop contract', async t => {
    const { dropzone } = t.context.accounts;
    let result = await dropzone.view('get_root_account', {});
    t.is(result, 'testnet');

    await dropzone.call(dropzone, 'set_root_account', {root_account: 'foo'});
    
    result = await dropzone.view('get_root_account', {});
    t.is(result, 'foo');
});

test('Setting gas price', async t => {
    const { dropzone } = t.context.accounts;
    let result = await dropzone.view('get_gas_price', {});
    t.is(result, '100000000');

    await dropzone.call(dropzone, 'set_gas_price', {yocto_per_gas: 100});
    
    result = await dropzone.view('get_gas_price', {});
    t.is(result, '100');
});

test('Deposit & withdraw to user balance', async t => {
    const { dropzone, ali } = t.context.accounts;
    let result = await dropzone.view('get_user_balance', {account_id: ali});
    t.is(result, '0');

    let b1 = await ali.availableBalance();
    await ali.call(dropzone, 'add_to_balance', {}, defaultCallOptions());
    let b2 = await ali.availableBalance();
    t.assert(assertBalanceChange(b1, b2, new NEAR(DEFAULT_DEPOSIT), 0.01), "balance didn't decrement properly with 1% precision");

    result = await dropzone.view('get_user_balance', {account_id: ali});
    t.is(result, DEFAULT_DEPOSIT);

    b1 = await ali.availableBalance();
    await ali.call(dropzone, 'withdraw_from_balance', {});
    b2 = await ali.availableBalance();
    t.assert(assertBalanceChange(b1, b2, new NEAR(DEFAULT_DEPOSIT), 0.01), "balance didn't increment properly with 1% precision");

    result = await dropzone.view('get_user_balance', {account_id: ali});
    t.is(result, '0');
});

test('Withdrawing fees earned', async t => {
    const { dropzone, ali } = t.context.accounts;
    let result = await dropzone.view('get_fees_collected', {});
    t.is(result, '0');
        
    await ali.call(dropzone, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    await ali.call(dropzone, 'create_drop', {public_keys: [], deposit_per_use: NEAR.parse('5 mN').toString()})
    
    result = await dropzone.view('get_fees_collected', {});
    t.is(result, NEAR.parse("1").toString());
    
    let keyPair = await KeyPairEd25519.fromRandom();
    await ali.call(dropzone, 'add_keys', {public_keys: [keyPair.publicKey.toString()], drop_id: 0})

    result = await dropzone.view('get_fees_collected', {});
    t.is(result, NEAR.parse('1005 mN').toString());

    result = await dropzone.call(dropzone, 'withdraw_fees', {withdraw_to: "no-exist.testnet"});
    result = await dropzone.view('get_fees_collected', {});
    t.is(result, NEAR.parse('1005 mN').toString());

    let b1 = await ali.availableBalance();
    result = await dropzone.call(dropzone, 'withdraw_fees', {withdraw_to: ali});
    result = await dropzone.view('get_fees_collected', {});
    t.is(result, NEAR.parse('0').toString());

    let b2 = await ali.availableBalance();
    t.assert(assertBalanceChange(b1, b2, NEAR.parse('1005 mN'), 0.01), "balance didn't decrement properly with 1% precision");
});

test('Custom fees earned', async t => {
    const { dropzone, ali } = t.context.accounts;
    let result = await dropzone.view('get_fees_collected', {});
    t.is(result, '0');
        
    await dropzone.call(dropzone, 'set_fees_per_user', {account_id: ali, drop_fee: NEAR.parse("5").toString(), key_fee: "0"});
    await ali.call(dropzone, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});
    await ali.call(dropzone, 'create_drop', {public_keys: [], deposit_per_use: NEAR.parse('5 mN').toString()})
    
    result = await dropzone.view('get_fees_collected', {});
    t.is(result, NEAR.parse("5").toString());
    
    let keyPair = await KeyPairEd25519.fromRandom();
    await ali.call(dropzone, 'add_keys', {public_keys: [keyPair.publicKey.toString()], drop_id: 0})

    result = await dropzone.view('get_fees_collected', {});
    t.is(result, NEAR.parse('5').toString());
});