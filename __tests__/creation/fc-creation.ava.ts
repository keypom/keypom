import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { claimWithRequiredGas, displayBalances, functionCall, generateKeyPairs } from "../utils/general";
import { FCData } from "@keypom/core";
const { readFileSync } = require('fs');

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    rpcPort: string;
  }>;

test.beforeEach(async (t) => {
    console.log(t.title);
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    const rpcPort = (worker as any).config.rpcAddr
    console.log(`rpcPort: `, rpcPort)
    
    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;
    console.log('root: ', root)
    
    const keypomV3 = await root.createSubAccount('keypom');
    console.log('keypomV3: ', keypomV3.accountId)

    // Test users
    const funder = await root.createSubAccount('funder');
    console.log('funder: ', funder.accountId)
    await funder.updateAccount({
        amount: NEAR.parse('10000000 N').toString()
    })
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    console.log('Deployed contracts: ', keypomV3.accountId)
    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"} });
    
    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, keypomV3 };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Initial Test', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    // will pass
    let method1 = {
        receiver_id: keypomV3.accountId,
        method_name: 'create_drop',
        args: JSON.stringify({

        }),
        attached_deposit: NEAR.parse("1").toString(),
        attached_gas: "10000000000000",
    }
    // will fail
    let method2 = {
        receiver_id: keypomV3.accountId,
        method_name: 'create_drop',
        args: JSON.stringify({}),
        attached_deposit: NEAR.parse("1").toString(),
        attached_gas: "10000000000000",
    }

    const fcAsset1 = [method1, method1, method1, method1]
    

    const dropId = "drop-id";
    const asset_data = [{
        assets: [fcAsset1, fcAsset1, fcAsset1],
        uses: 1
    }]

    let keyPairs = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0]
            }]
        },
        attachedDeposit: NEAR.parse("100").toString()
    })

    let result: {response: string | undefined, actualReceiverId: string | undefined} 
    = await claimWithRequiredGas({
        keypom: keypomV3,
        keyPair: keyPairs.keys[0],
        root,
        createAccount: true
    })
    t.is(result.response == "true", true)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});