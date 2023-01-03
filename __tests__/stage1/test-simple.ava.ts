import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, getDropInformation, getKeyInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
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

//testing drop empty initialization and that default values perform as expected
test('Create empty drop check views', async t => {
    const { keypom, ali } = t.context.accounts;
    //add 2NEAR to ali's keypom balance
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    //create a drop with Ali, doesn't front any cost. 
    await ali.call(keypom, 'create_drop', {deposit_per_use: NEAR.parse('5 mN').toString()});

    
    //store the results of all view functions into results
    let result = await queryAllViewFunctions({
        contract: keypom, 
        drop_id: "0", 
        account_id: ali.accountId
    });

    //pretty much all values should be 0
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

test('Testing Registered Uses Functionalities', async t => {
    const { keypom, ali, bob } = t.context.accounts;

    await ali.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    
    const simple: SimpleData = {
        lazy_register: true
    }
    let {keys, publicKeys} = await generateKeyPairs(100);
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), simple}, {gas: WALLET_GAS});
    let dropInfo = await getDropInformation(keypom,  "0");
    t.is(dropInfo.registered_uses, 0);

    // Reset the balance of bob to make sure they don't receive $$
    await bob.updateAccount({
        amount: "0"
    })

    await keypom.setKey(keys[0]);
    //give full access to the key above since failing a transaction would lead to not enough allowance on a regular function call access key
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )

    // THIS SHOULD FAIL SINCE NO KEYS ARE REGISTERED
    await keypom.call(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});

    let bobBal = await bob.availableBalance();
    console.log('aliBal Before: ', bobBal.toString())
    t.is(bobBal.toString(), NEAR.parse("0").toString());

    await ali.call(keypom, 'withdraw_from_balance', {});
    await ali.call(keypom, 'register_uses', {drop_id: "0", num_uses: 2}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse("150")});
    let aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.is(aliBal, NEAR.parse("148").toString());
    await ali.call(keypom, 'withdraw_from_balance', {});

    dropInfo = await getDropInformation(keypom,  "0");
    t.is(dropInfo.registered_uses, 2);

    // THIS SHOULD NOW PASS
    await keypom.call(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
    bobBal = await bob.availableBalance();
    console.log('Bob Bal Before: ', bobBal.toString())
    t.is(bobBal.toString(), NEAR.parse("1").toString());
    await ali.call(keypom, 'withdraw_from_balance', {});

    dropInfo = await getDropInformation(keypom,  "0");
    t.is(dropInfo.registered_uses, 1);

    try {
        await ali.call(keypom, 'unregister_uses', {drop_id: "0", num_uses: 100}, {gas: LARGE_GAS});
    } catch {}

    dropInfo = await getDropInformation(keypom,  "0");
    t.is(dropInfo.registered_uses, 1);

    aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.is(aliBal, "0");

    await ali.call(keypom, 'unregister_uses', {drop_id: "0", num_uses: 1}, {gas: LARGE_GAS});

    dropInfo = await getDropInformation(keypom,  "0");
    t.is(dropInfo.registered_uses, 0);

    aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.is(aliBal, NEAR.parse("1").toString());
});

test('Refunding Partially Registered Simple Drop', async t => {
    const { keypom, ali, bob } = t.context.accounts;
    
    const simple: SimpleData = {
        lazy_register: true
    }
    let {keys, publicKeys} = await generateKeyPairs(100);
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), simple}, {gas: WALLET_GAS});
    let aliBal: string = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    let diff = NEAR.parse('10').sub(NEAR.from(aliBal));
    console.log('diff: ', diff.toString())
    await ali.call(keypom, 'withdraw_from_balance', {});

    await ali.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});
    aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.assert(NEAR.from(aliBal).lte(diff));
});

test('Batch Creating Keys and Deleting All of Them With Partial Registration', async t => {
    const { keypom, ali, bob } = t.context.accounts;
    
    const simple: SimpleData = {
        lazy_register: true
    }

    // Set ali's balance to 1000 so we can check if the claim works properly
    // Add 10k $NEAR to owner's account
    await ali.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    
    let {keys, publicKeys: pks1} = await generateKeyPairs(100);
    await ali.call(keypom, 'create_drop', {public_keys: pks1, deposit_per_use: NEAR.parse('1').toString(), simple}, {gas: LARGE_GAS});
    let {publicKeys: pks2} = await generateKeyPairs(100);
    await ali.call(keypom, 'add_keys', {drop_id: "0", public_keys: pks2}, {gas: LARGE_GAS});
    let {publicKeys: pks3} = await generateKeyPairs(100);
    await ali.call(keypom, 'add_keys', {drop_id: "0", public_keys: pks3}, {gas: LARGE_GAS});
    
    let aliBal: string = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    let diff = NEAR.parse('100').sub(NEAR.from(aliBal));
    console.log('diff: ', diff.toString())
    await ali.call(keypom, 'withdraw_from_balance', {});

    await ali.call(keypom, 'register_uses', {drop_id: "0", num_uses: 150}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse("150")});
    aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    t.is(aliBal, "0");

    await ali.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});
    aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.assert(NEAR.from(aliBal).gte(NEAR.parse("100")));

    await ali.call(keypom, 'withdraw_from_balance', {});
    await ali.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});
    aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.assert(NEAR.from(aliBal).gte(NEAR.parse("50")) && NEAR.from(aliBal).lte(NEAR.parse("70")));

    await ali.call(keypom, 'withdraw_from_balance', {});
    await ali.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});
    aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.assert(NEAR.from(aliBal).lte(NEAR.parse("20")));
});

test('Refunding Over Registered Simple Drop', async t => {
    const { keypom, ali, bob } = t.context.accounts;
    
    const simple: SimpleData = {
        lazy_register: true
    }
        await ali.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

    let {keys, publicKeys} = await generateKeyPairs(5);
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});
        
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), simple}, {gas: WALLET_GAS});
    await ali.call(keypom, 'register_uses', {drop_id: "0", num_uses: 150}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse("150")});

    let dropInfo = await getDropInformation(keypom,  "0");
    t.is(dropInfo.registered_uses, 150);

    await ali.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});
    let aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.is(aliBal, NEAR.parse("160").toString());
});

test('Refunding Critically Registered Simple Drop', async t => {
    const { keypom, ali, bob } = t.context.accounts;
    
    const simple: SimpleData = {
        lazy_register: true
    }
        await ali.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

    let {keys, publicKeys} = await generateKeyPairs(5);
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});
        
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), simple}, {gas: WALLET_GAS});
    await ali.call(keypom, 'register_uses', {drop_id: "0", num_uses: 5}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse("5")});

    let dropInfo = await getDropInformation(keypom,  "0");
    t.is(dropInfo.registered_uses, 5);

    await ali.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});
    let aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    t.is(aliBal, NEAR.parse("15").toString());
});

test('Attempt to Panic During Claim or CAAC', async t => {
    const { keypom, ali, bob } = t.context.accounts;

    let keypomBalanceBefore = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalanceBefore.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalanceBefore.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalanceBefore.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalanceBefore.total.toString())

    await ali.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    
    let {keys, publicKeys} = await generateKeyPairs(1);
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("1000").toString()});

    let config: DropConfig = {
        uses_per_key: 1
    }
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), config},{gas: WALLET_GAS});

    await keypom.setKey(keys[0]);
    
    // Query for the access key's allowance
    let accessKeyInfo = await getKeyInformation(keypom, publicKeys[0]);
    console.log('accessKeyInfo: ', accessKeyInfo)
    
    // Drain contract by calling this 10 times
    for (let i = 0; i < config.uses_per_key!; i++) {
        try {
            await keypom.call(keypom, 'create_account_and_claim', {new_account_id: "foo", new_public_key: "foo"}, {gas: WALLET_GAS});
        }
        catch(e) {
            console.log(e);
        }
    }

    let accessKeyInfoAfter = await getKeyInformation(keypom, publicKeys[0]);
    console.log('accessKeyInfo After: ', accessKeyInfoAfter)

    t.assert(accessKeyInfoAfter.allowance < accessKeyInfo.allowance);

    // Delete the drop and withdraw all balance. Ensure keypom's available balance does not decrease
    await ali.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});

    let aliBal = await keypom.view('get_user_balance', {account_id: ali.accountId});
    console.log('aliBal: ', aliBal);
    await ali.call(keypom, 'withdraw_from_balance', {}, {gas: LARGE_GAS});

    let keypomBalanceAfter = await keypom.balance();
    console.log('keypom available AFTER: ', keypomBalanceAfter.available.toString())
    console.log('keypom staked AFTER: ', keypomBalanceAfter.staked.toString())
    console.log('keypom stateStaked AFTER: ', keypomBalanceAfter.stateStaked.toString())
    console.log('keypom total AFTER: ', keypomBalanceAfter.total.toString())

    t.assert(NEAR.from(keypomBalanceAfter.available.toString()).gte(NEAR.from(keypomBalanceBefore.available.toString())));
});