import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, getDropInformation, getKeySupplyForDrop, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, SimpleData } from "../utils/types";

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

    t.assert(jsonDrop.simple != undefined);
    t.assert(jsonDrop.nft == undefined);
    t.assert(jsonDrop.ft == undefined);
    t.assert(jsonDrop.fc == undefined);
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
    
    const dropConfig: DropConfig = {
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

test('Testing Lazy Registration', async t => {
    const { keypom, ali, bob } = t.context.accounts;
    
    const simple: SimpleData = {
        lazy_register: true
    }
        
    let {keys, publicKeys} = await generateKeyPairs(100);
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("5").toString()});
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), simple}, {gas: WALLET_GAS});
    
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 100);

    let dropInfo = await getDropInformation(keypom, "0");
    console.log('dropInfo: ', dropInfo)
    t.is(dropInfo.registered_uses, 0);

    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        keys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
        )
        

    // Set ali's balance to 0 so we can check if the claim works properly
    await bob.updateAccount({
        amount: "0"
    })
    // THIS SHOULD FAIL BECAUSE NO KEYS ARE REGISTERED
    await keypom.call(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});

    let bobBal = await bob.availableBalance();
    console.log('Bob Bal: ', bobBal.toString())
    t.is(bobBal.toString(), NEAR.parse("0").toString());

    // Withdraw balance
    await ali.call(keypom, 'withdraw_from_balance', {});
    await ali.call(keypom, 'register_uses', {drop_id: "0", num_uses: 15}, {attachedDeposit: NEAR.parse("15").toString()});

    let aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.is(aliBal, "0");

    dropInfo = await getDropInformation(keypom, "0");
    console.log('dropInfo: ', dropInfo)
    t.is(dropInfo.registered_uses, 15);

    // THIS SHOULD NOW PASS
    await keypom.call(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});

    bobBal = await bob.availableBalance();
    console.log('Bob Bal: ', bobBal.toString())
    t.is(bobBal.toString(), NEAR.parse("1").toString());

    dropInfo = await getDropInformation(keypom, "0");
    console.log('dropInfo: ', dropInfo)
    t.is(dropInfo.registered_uses, 14);
});