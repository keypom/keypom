import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, displayBalances, doesDropExist, doesKeyExist, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtNearData } from "../utils/types";
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
    
    const keypomV3 = await root.createSubAccount('keypom');
    // Test users
    const funder = await root.createSubAccount('funder');
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
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

test('Single NEAR Claim', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "NEAR Claim";
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }
    const asset_data = [
        {
            assets: [nearAsset1],
            uses: 1
        }
    ]
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
        attachedDeposit: NEAR.parse("10").toString()
    })
    let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)
    t.is(Object.keys(dropInfo.asset_data).length, 1);
    t.deepEqual(dropInfo.asset_data[0].assets, asset_data[0].assets)

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), true)

    let result: {response: string | undefined, actualReceiverId: string | undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        keyPair: keyPairs.keys[0],
        root,
        createAccount: true
    })
    t.is(result.response == "true", true)
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    t.is(await (keypomV3.getAccount(claimingAccount)).exists(), true);
    t.is(await doesDropExist(keypomV3, dropId), false)

    console.log(claimingAccount)

    let userBal: {available: NEAR} = await root.getAccount(claimingAccount).balance()
    console.log('userBal: ', userBal.available.toString());
    t.is(userBal.available.gte(NEAR.parse("1")), true)
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Multi NEAR Claim', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "Multi NEAR Claim";
    const nearAsset1 = {
        yoctonear: NEAR.parse("1").toString()
    }
    const nearAsset2 = {
        yoctonear: NEAR.parse("3").toString()
    }
    const nearAsset3 = {
        yoctonear: NEAR.parse("5").toString()
    }
    const asset_data = [
        {
            // 7 NEAR
            assets: [nearAsset1, nearAsset1, nearAsset3,],
            uses: 1,
            config: null
        },
        {
            // 9 NEAR
            assets: [nearAsset1, nearAsset2, nearAsset3],
            uses: 1,
            config: null
        },
    ]
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
        attachedDeposit: NEAR.parse("20").toString()
    })
    let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)
    t.deepEqual(dropInfo.asset_data, asset_data)

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), true)

    // FIRST KEY USE
    let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining == 2, true)

    let result: {response: string | undefined, actualReceiverId: string | undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        keyPair: keyPairs.keys[0],
        root,
        createAccount: true
    })
    t.is(result.response == "true", true)
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    t.is(await (keypomV3.getAccount(claimingAccount)).exists(), true);
    t.is(await doesDropExist(keypomV3, dropId), true)

    let userBal: {available: NEAR} = await root.getAccount(claimingAccount).balance()
    console.log('userBal: ', userBal.available.toString());
    t.is(userBal.available.gte(NEAR.parse("7")), true)

    // SECOND KEY USE
    keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining == 1, true)

    result = await claimWithRequiredGas({
        keypom: keypomV3,
        keyPair: keyPairs.keys[0],
        root,
        createAccount: true
    })
    t.is(result.response == "true", true)
    claimingAccount = result.actualReceiverId == undefined ? "" : result.actualReceiverId
    t.is(await (keypomV3.getAccount(claimingAccount)).exists(), true);

    userBal = await root.getAccount(claimingAccount).balance()
    console.log('userBal: ', userBal.available.toString());
    t.is(userBal.available.gte(NEAR.parse("9")), true)

    // Drop should be depleted and deleted
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]) ,false)
    t.is(await doesDropExist(keypomV3, dropId), false)
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

