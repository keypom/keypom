import { Worker, NearAccount, NEAR } from "near-workspaces";
import anyTest, { TestFn } from "ava";
import { assertBalanceChange, defaultCallOptions, DEFAULT_DEPOSIT } from "./utils/utils";

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
    await dropzone.call(dropzone, 'new', {linkdrop_contract: 'testnet', owner_id: dropzone});

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
    const result = await dropzone.view('get_nonce', {});
    t.is(result, 0);
});

test('deposit & withdraw to user balance', async t => {
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

// test('deposit funds to user balance', async t => {
//     const { dropzone, ali } = t.context.accounts;
//     let result = await dropzone.view('get_user_balance', {account_id: ali});
//     t.is(result, '0');

//     const foo = await ali.call(dropzone, 'add_to_balance', {}, defaultCallOptions());
//     console.log('foo: ', foo)

//     result = await dropzone.view('get_user_balance', {account_id: ali});
//     t.is(result, DEFAULT_DEPOSIT);
// });