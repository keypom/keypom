import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, LARGE_GAS } from "../utils/general";
import { DropConfig } from "../utils/types";

const path = require("path");
const homedir = require("os").homedir();
const { writeFile, mkdir, readFile } = require('fs/promises');

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    keypomInitialBalance: NEAR;
    keypomInitialStateStaked: NEAR;
}>;


test.beforeEach(async (t) => {
    // Comment this if you want to see console logs
    //console.log = function() {}
    console.log("starting")

    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy all 3 contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    
    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    
    // Custom root
    const customRoot = await root.createSubAccount('custom-root');
    await customRoot.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    await customRoot.call(customRoot, 'new', {});

    // Add 10k $NEAR to owner's account
    await owner.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, owner, ali, customRoot };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Simple Drop Upfront', async t => {
    const { keypom, owner, ali } = t.context.accounts;
    let startIndex = 0;
    let finishIndex = 100;

    // dataToWrite is an object containing strings that map to objects
    let dataToWrite: Record<string, Record<string, string>> = {};

    let config: DropConfig = {
        usage: {
            auto_withdraw: true,
            auto_delete_drop: true
        }
    }
    
    // Loop through and create a drop with 0 all the way to 100 keys per drop and check the net user costs
    for (let i = startIndex; i < finishIndex; i++) {
        let {keys, publicKeys} = await generateKeyPairs(i+1);

        // Withdraw all balance and deposit 1000 $NEAR
        await owner.call(keypom, 'withdraw_from_balance', {});
        await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("1000").toString()});

        let bal1 = await owner.balance();
        // Creating the drop that should be deleted
        await owner.call(keypom, 'create_drop', {
            public_keys: publicKeys, 
            deposit_per_use: NEAR.parse("1").toString(),
            config,
        },{gas: LARGE_GAS});
        let bal2 = await owner.balance();

        let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner});
        let netCost = NEAR.parse("1000").sub(NEAR.from(ownerBal));
        
        dataToWrite[`${i}-keys`] = {
            "initialActual": bal1.available.toString(),
            "finalActual": bal2.available.toString(),
            "netActual": bal1.available.sub(bal2.available).toString(),
            "initialBalance": NEAR.parse("1000").toString(),
            "finalBalance": ownerBal,
            "netCost": netCost.toString(),
        }
    }

    await writeFile(path.resolve(__dirname, `simple.json`), JSON.stringify(dataToWrite));
});

test('Simple Drop NET', async t => {
    const { keypom, owner, ali } = t.context.accounts;
    let startIndex = 0;
    let finishIndex = 1;

    // dataToWrite is an object containing strings that map to objects
    let dataToWrite: Record<string, Record<string, string>> = {};

    let config: DropConfig = {
        usage: {
            auto_withdraw: true,
            auto_delete_drop: true
        }
    }
    
    // Loop through and create a drop with 0 all the way to 100 keys per drop and check the net user costs
    for (let i = startIndex; i < finishIndex; i++) {
        let {keys, publicKeys} = await generateKeyPairs(i+1);

        // Withdraw all balance and deposit 1000 $NEAR
        await owner.call(keypom, 'withdraw_from_balance', {});
        await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("1000").toString()});

        let bal1 = await owner.balance();
        // Creating the drop that should be deleted
        await owner.call(keypom, 'create_drop', {
            public_keys: publicKeys, 
            deposit_per_use: NEAR.parse("1").toString(),
            config,
        },{gas: LARGE_GAS});
        let bal2 = await owner.balance();

        let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner});
        let netCost = NEAR.parse("1000").sub(NEAR.from(ownerBal));
        
        dataToWrite[`${i}-keys`] = {
            "initialActual": bal1.available.toString(),
            "finalActual": bal2.available.toString(),
            "netActual": bal1.available.sub(bal2.available).toString(),
            "initialBalance": NEAR.parse("1000").toString(),
            "finalBalance": ownerBal,
            "netCost": netCost.toString(),
        }
    }

    await writeFile(path.resolve(__dirname, `simple.json`), JSON.stringify(dataToWrite));
});