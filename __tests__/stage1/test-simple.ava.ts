import { Worker, NearAccount, NEAR, KeyPairEd25519, tGas } from "near-workspaces";
import anyTest, { TestFn } from "ava";
import { assertBalanceChange, defaultCallOptions, DEFAULT_DEPOSIT, DEFAULT_GAS, queryAllViewFunctions } from "../utils/utils";
import { JsonDrop } from "../utils/types";

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

test('Create empty drop check views', async t => {
    const { dropzone, ali } = t.context.accounts;
    
    await ali.call(dropzone, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    await ali.call(dropzone, 'create_drop', {public_keys: [], deposit_per_use: NEAR.parse('5 mN').toString()});
    
    let result = await queryAllViewFunctions({
        contract: dropzone, 
        drop_id: 0, 
        account_id: ali.accountId
    });

    t.is(result.keyTotalSupply, '0');
    t.deepEqual(result.keys, []);
    let jsonDrop = result.dropInformation!;
    t.is(jsonDrop.drop_id, 0);
    t.is(jsonDrop.owner_id, ali.accountId);
    t.is(jsonDrop.deposit_per_use, NEAR.parse('5 mN').toString());
    t.is(jsonDrop.drop_type, 'Simple');
    t.is(jsonDrop.config, null);
    t.is(jsonDrop.metadata, null);
    t.is(jsonDrop.registered_uses, 0);
    t.is(jsonDrop.required_gas, tGas(100));
    t.is(jsonDrop.next_key_id, 0);
    
    t.is(result.keySupplyForDrop, 0);
    t.deepEqual(result.keysForDrop, []);
    t.deepEqual(result.tokenIdsForDrop, []);
    t.deepEqual(result.keySupplyForOwner, 0);
    t.deepEqual(result.dropSupplyForOwner, 1);
});

test('Create drop with 1000 keys', async t => {
    const { dropzone, ali } = t.context.accounts;
    
    // log ali's available balance
    let b = await ali.availableBalance();
    console.log('b: ', b.toHuman())
});