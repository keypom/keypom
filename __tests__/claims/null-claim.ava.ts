import anyTest, { TestFn } from "ava";
import { claimTrialAccountDrop, createDrop, createTrialAccountDrop, getDrops, getUserBalance, parseNearAmount, trialCallMethod } from "keypom-js";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, displayBalances, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop } from "../utils/types";
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
    
    await keypomV3.deploy(`./out/mapping.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId });
    
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

interface FTAsset {
    contract_id: string;
    registration_cost: string;
    amount: string;
}

test('Single Null Claim', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "Null Claim";
    const assets_per_use = {
        1: [null],
    }
    let keyPairs = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_per_use,
            public_keys: keyPairs.publicKeys
        },
        attachedDeposit: NEAR.parse("10").toString()
    })
    let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)
    t.is(dropInfo.internal_assets_data.length, 1);
    t.is(dropInfo.internal_assets_data[0], null);

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)
    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 2);

    await keypomV3.setKey(keyPairs.keys[0]);
    let newAccountId = `new-account.${root.accountId}`;
    let keyPk = keyPairs.publicKeys[0];
    const keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPk});
    console.log('keyInfo: ', keyInfo)
    
    await functionCall({
        signer: keypomV3,
        receiver: keypomV3,
        methodName: 'create_account_and_claim',
        args: {new_account_id: newAccountId, new_public_key: keyPk},
        gas: keyInfo.required_gas
    })

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    let doesExist = await (keypomV3.getAccount(newAccountId)).exists();
    console.log('doesExist: ', doesExist)
    t.is(doesExist, true);

    try {
        const keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPk});
        console.log('keyInfo (Should have panicked): ', keyInfo)
        t.fail('Key should have been deleted');
    } catch (e) {
        t.pass();
    }

    let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo (after drop is deleted): ', endingDropInfo)
    t.is(endingDropInfo, null);
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Multi Null Claim', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "Null Claim";
    const assets_per_use = {
        1: [null, null, null, null, null, null, null, null, null],
        2: [null, null, null, null, null, null, null, null, null],
        3: [null, null, null, null, null, null, null, null, null],
    }
    let keyPairs = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_per_use,
            public_keys: keyPairs.publicKeys
        },
        attachedDeposit: NEAR.parse("10").toString()
    })
    let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)
    t.is(dropInfo.internal_assets_data.length, 1);
    t.is(dropInfo.internal_assets_data[0], null);

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)
    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 2);

    await keypomV3.setKey(keyPairs.keys[0]);
    let newAccountId = `new-account.${root.accountId}`;
    let keyPk = keyPairs.publicKeys[0];
    let keyInfo: {required_gas: string, uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPk});
    console.log('keyInfo: ', keyInfo)
    t.is(keyInfo.uses_remaining, 3);
    
    await functionCall({
        signer: keypomV3,
        receiver: keypomV3,
        methodName: 'claim',
        args: {account_id: newAccountId},
        gas: keyInfo.required_gas
    })

    keyInfo = await keypomV3.view('get_key_information', {key: keyPk});
    console.log('keyInfo: ', keyInfo)
    t.is(keyInfo.uses_remaining, 2);

    await functionCall({
        signer: keypomV3,
        receiver: keypomV3,
        methodName: 'claim',
        args: {account_id: newAccountId},
        gas: keyInfo.required_gas
    })
    
    keyInfo = await keypomV3.view('get_key_information', {key: keyPk});
    console.log('keyInfo: ', keyInfo)
    t.is(keyInfo.uses_remaining, 1);
    
    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 2);

    await functionCall({
        signer: keypomV3,
        receiver: keypomV3,
        methodName: 'create_account_and_claim',
        args: {new_account_id: newAccountId, new_public_key: keyPk},
        gas: keyInfo.required_gas
    })

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    let doesExist = await (keypomV3.getAccount(newAccountId)).exists();
    console.log('doesExist: ', doesExist)
    t.is(doesExist, true);

    try {
        const keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPk});
        console.log('keyInfo (Should have panicked): ', keyInfo)
        t.fail('Key should have been deleted');
    } catch (e) {
        t.pass();
    }

    let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo (after drop is deleted): ', endingDropInfo)
    t.is(endingDropInfo, null);
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Tons of null claims', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "Null Claim";
    const assets_per_use = {
        1: [
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
        ],
    }
    let keyPairs = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_per_use,
            public_keys: keyPairs.publicKeys
        },
        attachedDeposit: NEAR.parse("10").toString()
    })
    let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)
    t.is(dropInfo.internal_assets_data.length, 1);
    t.is(dropInfo.internal_assets_data[0], null);

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)
    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 2);

    await keypomV3.setKey(keyPairs.keys[0]);
    let newAccountId = `new-account.${root.accountId}`;
    let keyPk = keyPairs.publicKeys[0];
    let keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPk});
    console.log('keyInfo: ', keyInfo)
    
    await functionCall({
        signer: keypomV3,
        receiver: keypomV3,
        methodName: 'create_account_and_claim',
        args: {new_account_id: newAccountId, new_public_key: keyPk},
        gas: keyInfo.required_gas
    })

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    let doesExist = await (keypomV3.getAccount(newAccountId)).exists();
    console.log('doesExist: ', doesExist)
    t.is(doesExist, true);

    try {
        const keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPk});
        console.log('keyInfo (Should have panicked): ', keyInfo)
        t.fail('Key should have been deleted');
    } catch (e) {
        t.pass();
    }

    let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo (after drop is deleted): ', endingDropInfo)
    t.is(endingDropInfo, null);
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});