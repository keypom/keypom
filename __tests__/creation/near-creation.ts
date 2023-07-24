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

// test('Panic Ensure No Keys Added (duplicate keys)', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [null],
//     }
//     let keyPairs = await generateKeyPairs(70);
//     keyPairs.publicKeys.push(keyPairs.publicKeys[0]);

//     // First 70 are good and the last is repeated. No keys should be added
//     // And the contract should panic
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys
//         },
//         attachedDeposit: NEAR.parse("10").toString(),
//         shouldPanic: true
//     })

//     try {
//         let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//         console.log('keysForDrop: ', keysForDrop)
//         t.fail('Should have panicked')
//     } catch (e) {
//         t.pass()
//     }

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

//     // // PK already exists on the contract
//     // await functionCall({
//     //     signer: funder,
//     //     receiver: keypomV3,
//     //     methodName: 'add_keys',
//     //     args: {
//     //         drop_id: dropId,
//     //         public_keys: keyPairs.publicKeys
//     //     },
//     //     attachedDeposit: NEAR.parse("10").toString()
//     // })

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
//     t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
// });

// test('Panic Ensure No Keys Added (too many actions)', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [null],
//     }
//     let keyPairs = await generateKeyPairs(101);

//     // There are more than 100 actions so the contract should panic
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys
//         },
//         attachedDeposit: NEAR.parse("10").toString(),
//         shouldPanic: true
//     })

//     try {
//         let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//         console.log('keysForDrop: ', keysForDrop)
//         t.fail('Should have panicked')
//     } catch (e) {
//         t.pass()
//     }

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
//     t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
// });

// test('Adding Keys', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();


//     const ftAsset: FTAsset = {
//         contract_id: "foo.bar",
//         registration_cost: NEAR.parse("1").toString(),
//         amount: NEAR.parse("1").toString()
//     }

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [ftAsset],
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

//     keyPairs = await generateKeyPairs(1);
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'add_keys',
//         args: {
//             drop_id: dropId,
//             public_keys: keyPairs.publicKeys
//         },
//         attachedDeposit: NEAR.parse("10").toString(),
//     })

//     keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 2)

//     keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 3);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });

// test('Not enough $NEAR attached', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nearAsset1 = {
//         yoctonear: NEAR.parse("1").toString()
//     }

//     const nearAsset2 = {
//         yoctonear: NEAR.parse("2").toString()
//     }

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [nearAsset1],
//         2: [nearAsset1, nearAsset2],
//         3: [nearAsset1, nearAsset2, nearAsset2],
//     }

//     let keyPairs = await generateKeyPairs(70);
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys
//         },
//         attachedDeposit: NEAR.parse("10").toString(),
//         shouldPanic: true
//     })

//     try {
//         let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//         console.log('keysForDrop: ', keysForDrop)
//         t.fail('Should have panicked')
//     } catch (e) {
//         t.pass()
//     }

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
//     t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
// });

test('Not enough $NEAR attached', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nearAsset1 = {
        yoctonear: NEAR.parse("1").toString()
    }
 
    const nearAsset2 = {
        yoctonear: NEAR.parse("2").toString()
    }
    
    const ftAsset1 = {
        contract_id: "foo.bar",
        registration_cost: NEAR.parse("1").toString(),
        amount: NEAR.parse("1").toString()
    }

    const dropId = "drop-id";
    const assets_per_use = {
        1: [nearAsset1, ftAsset1, null]
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

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)

    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 2);

    let dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});