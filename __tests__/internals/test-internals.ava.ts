import anyTest, { TestFn } from "ava";
import { KeyPairEd25519, NEAR, NearAccount, Worker } from "near-workspaces";
import { assertBalanceChange, CONTRACT_METADATA, defaultCallOptions, DEFAULT_DEPOSIT } from "../utils/general";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
  }>;

test.beforeEach(async (t) => {
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy the keypom contract.
    const keypom = await root.devDeploy(`./out/keypom.wasm`);

    // Init the contract
    await keypom.call(keypom, 'new', {root_account: 'testnet', owner_id: keypom, contract_metadata: CONTRACT_METADATA});

    // Test users
    const ali = await root.createSubAccount('ali');
    const bob = await root.createSubAccount('bob');

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Initial nonce is 0', async t => {
    const { keypom } = t.context.accounts;
    const result = await keypom.view('get_next_drop_id', {});
    t.is(result, 0);
});

test('Changing linkdrop contract', async t => {
    const { keypom } = t.context.accounts;
    let result = await keypom.view('get_root_account', {});
    t.is(result, 'testnet');

    await keypom.call(keypom, 'set_root_account', {root_account: 'foo'});
    
    result = await keypom.view('get_root_account', {});
    t.is(result, 'foo');
});

test('Setting Contract Metadata', async t => {
    const { keypom } = t.context.accounts;
    let result = await keypom.view('contract_source_metadata', {});
    t.deepEqual(result, CONTRACT_METADATA);

    let newMetadata = {
        "version": "0.0.1",
        "link": "foo"
    }

    await keypom.call(keypom, 'set_contract_metadata', {contract_metadata: newMetadata});
    
    result = await keypom.view('contract_source_metadata', {});
    t.deepEqual(result, newMetadata);
});

test('Setting gas price', async t => {
    const { keypom } = t.context.accounts;
    let result = await keypom.view('get_gas_price', {});
    t.is(result, 100000000);

    await keypom.call(keypom, 'set_gas_price', {yocto_per_gas: 100});
    
    result = await keypom.view('get_gas_price', {});
    t.is(result, 100);
});

test('Deposit & withdraw to user balance', async t => {
    const { keypom, ali } = t.context.accounts;
    let result = await keypom.view('get_user_balance', {account_id: ali});
    t.is(result, '0');

    let b1 = await ali.availableBalance();
    await ali.call(keypom, 'add_to_balance', {}, defaultCallOptions());
    let b2 = await ali.availableBalance();
    t.assert(assertBalanceChange(b1, b2, new NEAR(DEFAULT_DEPOSIT), 0.01), "balance didn't decrement properly with 1% precision");

    result = await keypom.view('get_user_balance', {account_id: ali});
    t.is(result, DEFAULT_DEPOSIT);

    b1 = await ali.availableBalance();
    await ali.call(keypom, 'withdraw_from_balance', {});
    b2 = await ali.availableBalance();
    t.assert(assertBalanceChange(b1, b2, new NEAR(DEFAULT_DEPOSIT), 0.01), "balance didn't increment properly with 1% precision");

    result = await keypom.view('get_user_balance', {account_id: ali});
    t.is(result, '0');
});

test('Withdrawing fees earned', async t => {
    const { keypom, ali } = t.context.accounts;
    let result = await keypom.view('get_fees_collected', {});
    t.is(result, '0');
    await keypom.call(keypom, 'set_fees', {drop_fee: NEAR.parse("1").toString(), key_fee: NEAR.parse('5 mN').toString()});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    await ali.call(keypom, 'create_drop', {public_keys: [], deposit_per_use: NEAR.parse('5 mN').toString()})
    
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse("1").toString());
    
    let keyPair = await KeyPairEd25519.fromRandom();
    await ali.call(keypom, 'add_keys', {public_keys: [keyPair.publicKey.toString()], drop_id: 0})

    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse('1005 mN').toString());

    result = await keypom.call(keypom, 'withdraw_fees', {withdraw_to: "no-exist.testnet"});
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse('1005 mN').toString());

    let b1 = await ali.availableBalance();
    result = await keypom.call(keypom, 'withdraw_fees', {withdraw_to: ali});
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse('0').toString());

    let b2 = await ali.availableBalance();
    t.assert(assertBalanceChange(b1, b2, NEAR.parse('1005 mN'), 0.01), "balance didn't decrement properly with 1% precision");
});

test('Custom fees earned', async t => {
    const { keypom, ali } = t.context.accounts;
    let result = await keypom.view('get_fees_collected', {});
    t.is(result, '0');
        
    await keypom.call(keypom, 'set_fees_per_user', {account_id: ali, drop_fee: NEAR.parse("5").toString(), key_fee: "0"});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});
    await ali.call(keypom, 'create_drop', {public_keys: [], deposit_per_use: NEAR.parse('5 mN').toString()})
    
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse("5").toString());
    
    let keyPair = await KeyPairEd25519.fromRandom();
    await ali.call(keypom, 'add_keys', {public_keys: [keyPair.publicKey.toString()], drop_id: 0})

    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse('5').toString());
});