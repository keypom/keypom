import anyTest, { TestFn } from "ava";
import { formatNearAmount } from "near-api-js/lib/utils/format";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, displayBalances, doesDropExist, doesKeyExist, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData } from "../utils/types";
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

test('Panic Ensure No Keys Added (too many actions)', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "drop-id";
    const asset_data = [{
        assets: [null],
        uses: 1
    }]
    let numKeys = 101
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    // There are more than 100 actions so the contract should panic
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data
        },
        attachedDeposit: NEAR.parse("10").toString(),
        shouldPanic: true
    })

    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

test('Adding Keys', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();


    const ftAsset: ExtFTData = {
        ft_contract_id: "foo.bar",
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: NEAR.parse("1").toString()
    }

    const dropId = "drop-id";
    const asset_data = [{
        assets: [ftAsset],
        uses: 1
    }]
    let numKeys = 1
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data
        },
        attachedDeposit: NEAR.parse("10").toString()
    })

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)

    t.is(await doesDropExist(keypomV3, dropId), true)
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), true)

    numKeys = 5
    keyPairs = await generateKeyPairs(numKeys);
    key_data = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data
        },
        attachedDeposit: NEAR.parse("10").toString(),
    })

    keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 6)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Not enough $NEAR attached', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nearAsset1 = {
        yoctonear: NEAR.parse("1").toString()
    }

    const nearAsset2 = {
        yoctonear: NEAR.parse("2").toString()
    }

    const dropId = "drop-id";
    const assets_data = [
        {
            assets: [nearAsset1],
            uses: 1
        },
        {
            assets: [nearAsset1, nearAsset2],
            uses: 1
        },
        {
            assets: [nearAsset1, nearAsset2, nearAsset2],
            uses: 1
        },
    ]

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "withdraw_from_balance",
        args: {}
    })

    let userBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
    t.is(userBal == "0", true)

    let numKeys = 70
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_data,
            key_data
        },
        attachedDeposit: NEAR.parse("10").toString(),
        shouldPanic: true
    })

    try {
        let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
        console.log('keysForDrop: ', keysForDrop)
        t.fail('Should have panicked')
    } catch (e) {
        t.pass()
    }

    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

// test('Not enough $NEAR attached', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nearAsset1 = {
//         yoctonear: NEAR.parse("1").toString()
//     }
 
//     const nearAsset2 = {
//         yoctonear: NEAR.parse("2").toString()
//     }
    
//     const ftAsset1 = {
//         contract_id: "foo.bar",
//         registration_cost: NEAR.parse("1").toString(),
//         amount: NEAR.parse("1").toString()
//     }

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [nearAsset1, ftAsset1, null]
//     }

//     let keyPairs = await generateKeyPairs(1);
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys
//         },
//         attachedDeposit: NEAR.parse("10").toString()
//     })

//     let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 1)

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 2);

//     let dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo: ', dropInfo)

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });

// test('Not enough $NEAR attached', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nearAsset1 = {
//         yoctonear: NEAR.parse("1").toString()
//     }
 
//     //const nearAsset2 = {
//     //     yoctonear: NEAR.parse("2").toString()
//     // }
    
//     // const ftAsset1 = {
//     //     contract_id: "foo.bar",
//     //     registration_cost: NEAR.parse("1").toString(),
//     //     amount: NEAR.parse("1").toString()
//     // }

//     const dropId = "drop-id";
//     // Create an object with keys from 1 -> 1000 and every value is [nearAsset1]
//     const assets_per_use = {
//         ...Array.from(Array(1000).keys()).reduce((acc, cur) => {
//             acc[cur + 1] = [nearAsset1]
//             return acc
//         }
//         , {})
//     }
//     console.log('assets_per_use: ', assets_per_use)

//     let keyPairs = await generateKeyPairs(1);
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys
//         },
//         attachedDeposit: NEAR.parse("10").toString()
//     })

//     let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 1)

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 2);

//     let dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo: ', dropInfo)

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });