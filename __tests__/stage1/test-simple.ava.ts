import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, queryAllViewFunctions, WALLET_GAS } from "../utils/general";

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

test('Create empty drop check views', async t => {
    const { keypom, ali } = t.context.accounts;
    
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    await ali.call(keypom, 'create_drop', {public_keys: [], deposit_per_use: NEAR.parse('5 mN').toString()});
    
    let result = await queryAllViewFunctions({
        contract: keypom, 
        drop_id: "0", 
        account_id: ali.accountId
    });

    t.is(result.keyTotalSupply, 0);
    t.deepEqual(result.keys, []);
    let jsonDrop = result.dropInformation!;
    t.is(jsonDrop.drop_id, '0');
    t.is(jsonDrop.owner_id, ali.accountId);
    t.is(jsonDrop.deposit_per_use, NEAR.parse('5 mN').toString());
    t.is(jsonDrop.drop_type.toString(), 'Simple');
    t.is(jsonDrop.config, null);
    t.is(jsonDrop.metadata, null);
    t.is(jsonDrop.registered_uses, 0);
    t.is(jsonDrop.required_gas, tGas(100));
    t.is(jsonDrop.next_key_id, 0);
    
    t.is(result.keySupplyForDrop, 0);
    t.deepEqual(result.keysForDrop, []);
    t.deepEqual(result.tokenIdsForDrop, []);
    t.deepEqual(result.dropSupplyForOwner, 1);
});

test('Create drop with 1000 keys', async t => {
    const { keypom, ali } = t.context.accounts;
    
    const dropConfig = {
        uses_per_key: 1,
    }
        
    let {keys, publicKeys} = await generateKeyPairs(1);
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    await ali.call(keypom, 'create_drop', {public_keys: [], deposit_per_use: NEAR.parse('5 mN').toString(), config: dropConfig});
    await ali.call(keypom, 'add_keys', {drop_id: '0', public_keys: [publicKeys[0]]});

    let foo = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('foo: ', foo)
    await ali.call(keypom, 'withdraw_from_balance', {});
    foo = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('foo: ', foo)
    
    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        keys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
        )
        
    let {keys: keys2, publicKeys: publicKeys2} = await generateKeyPairs(1);
    await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `foo.test.near`, new_public_key : publicKeys[0]}, {gas: WALLET_GAS});
    foo = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('foo: ', foo)
});