import anyTest, { TestFn } from "ava";
import { claimTrialAccountDrop, createDrop, createTrialAccountDrop, getDrops, getUserBalance, parseNearAmount, trialCallMethod } from "keypom-js";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, displayBalances, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, InternalNFTData } from "../utils/types";
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
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });

    await functionCall({
        signer: funder,
        receiver: nftContract,
        methodName: 'nft_mint',
        args: {
            token_id: "token1",
            metadata: {
                title: "my token"
            },
            receiver_id: funder.accountId
        },
        attachedDeposit: NEAR.parse("1").toString(),
        shouldLog: false
    })

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_to_balance',
        args: {},
        attachedDeposit: NEAR.parse("10").toString(),
        shouldLog: false
    })
    
    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, keypomV3, nftContract };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Not enough balance / deposit
// test('Insufficient Balance + Deposit', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "drop-id";
//     let keyPairs = await generateKeyPairs(5);

//     const nearAsset1 = {
//         yoctonear: NEAR.parse("10").toString()
//     }
//     const assets_per_use = {
//         1: [nearAsset1],
//     }

//     let funderBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
//     t.assert(NEAR.parse(funderBal).lt(NEAR.from("10")));

//     // 5*10NEAR per use; balance + deposit must be more than 50 NEAR. This will panic
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

//     // Checking no keys were added, balances did not change etc. 
//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
//     t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
// });

// Conflicting public keys
// test('Conflicting Keys', async t => {
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

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
//     t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
// });

// Conflicting drop IDs
// test('Conflicting DropIDs', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [null],
//     }
//     let keyPairs = await generateKeyPairs(5);

//     // Add 5 keys
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
//         shouldPanic: false
//     })

//     // 1 FAK, 5 newly added LAK
//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 6);

//     let initialBadBal = await keypomV3.balance();

//     // Creating second drop with identical drop ID
//     let keyPairs2 = await generateKeyPairs(5);
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs2.publicKeys
//         },
//         attachedDeposit: NEAR.parse("10").toString(),
//         shouldPanic: false
//     })

//     // Keys should not have changed from last time since drop creation has failed
//     let keypomKeys2 = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys2.keys.length, 6);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBadBal, finalBal);
//     t.deepEqual(finalBal.stateStaked, initialBadBal.stateStaked);
// });

// Really really really long drop ID
// test('Really long DropIDs', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     // Massive Drop ID, this far exceeds the 2048Byte limit and will panic
//     const dropId = "ying shoelaces is a fundamental skill that most people learn at an early age, and it is an important step towards becoming self-sufficient. Although it may seem like a simple task, mastering the art of tying shoes can ensure a secure and comfortable fit throughout the day. Here's a step-by-step guide to help you tie your shoes with ease and confidence. Start by holding one shoelace in each hand, making sure the lengths are equal. Cross the laces over each other, creating an X shape. Take the lace on the right side and pass it over the lace on the left side, threading it through the space between the two laces. Now, pull both ends of the laces firmly to tighten the knot, ensuring the knot is positioned near the base of the shoelaces. Congratulations, you've successfully tied the first part! Moving on, create two loops with the laces, holding one loop in each hand. Cross the loops over each other, forming another X shape. Take the loop in your right hand and pass it over the loop in your left hand, threading it through the space between the loops. Hold the loops firmly and pull them outward to tighten the knot, making sure the loops are of equal size. This is the crucial moment when your shoes start taking shape. To complete the process, grab the loops and give them a slight tug to secure the knot firmly. Optionally, you can tuck the loops under the shoelace knot for a cleaner look and to prevent tripping. Now, you're ready to confidently walk, run, or engage in any activity without worrying about loose shoelaces. Congratulations, you've mastered the basic bunny ears method! However, if you prefer an alternative method, you can try the Ian knot method, which is faster but may take some time to learn. Here's a brief overview of the Ian knot: Start by crossing the laces over each other, just like in the previous method. Take the lace on the right side and pass it under the lace on the left side. Next, bring the end of the right lace through the space between the laces, creating a loop aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!";
//     const assets_per_use = {
//         1: [null],
//     }
//     let keyPairs = await generateKeyPairs(5);

//     // Add 5 keys
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys
//         },
//         attachedDeposit: NEAR.parse("1").toString(),
//         shouldPanic: true
//     })

//     // Keys should not have changed from last time since drop creation has failed
//     let keypomKeys2 = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys2.keys.length, 1);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
//     t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
// });


// Invalid args passed in (use number out of bounds)
test('Undefined Uses', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    // Massive Drop ID, this far exceeds the 2048Byte limit and will panic
    const dropId = "drop-id";
    // Uses missing 4 and 5 - 3 are swapped
    const assets_per_use = {
        1: [null],
        2: [null],
        5: [null],
        3: [null],
        // Adding this line will cause the drop creation to pass - order does not matter, simply that there are no missing uses
        // 4: [null],
    }
    let keyPairs = await generateKeyPairs(2);

    // Add 2 keys
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_per_use,
            public_keys: keyPairs.publicKeys
        },
        attachedDeposit: NEAR.parse("1").toString(),
        shouldPanic: true
    })

    // Keys should not have changed from last time since drop creation has failed
    let keypomKeys2 = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys2.keys.length, 1);

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

// Too many assets in a use leading to gas problems (try to force a panic as late down the road as possible i.e first 15 uses are fine but last one is not).

// Create function call drop with invalid permissions (pointing to keypom, invalid method names etc…)


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


// test('Simple NFT Creation', async t => {
//     const {funder, keypomV3, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nftAsset1 = {
//         nft_contract_id: "foobar.test.near"
//     }

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [nftAsset1]
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

//     let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log(`dropInfo: ${JSON.stringify(dropInfo)}`)
//     t.is(dropInfo.internal_assets_data.length, 1);
//     t.is((dropInfo.internal_assets_data[0] as InternalNFTData).nft.token_ids.length, 0);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });

// test('Add NFTs to created drop', async t => {
//     const {funder, keypomV3, nftContract, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nftAsset1 = {
//         nft_contract_id: nftContract.accountId
//     }

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [nftAsset1]
//     }

//     let keyPairs = await generateKeyPairs(1);
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys,
//         },
//         attachedDeposit: NEAR.parse("10").toString()
//     })

//     await functionCall({
//         signer: funder,
//         receiver: nftContract,
//         methodName: 'nft_transfer_call',
//         args: {
//             receiver_id: keypomV3.accountId,
//             token_id: "token1",
//             msg: dropId
//         },
//         attachedDeposit: "1"
//     })

//     let userBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
//     t.assert(NEAR.parse(userBal).lt(NEAR.from("10")));

//     let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 1)

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 2);

//     let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log(`dropInfo: ${JSON.stringify(dropInfo)}`)
//     t.is(dropInfo.internal_assets_data.length, 1);
//     t.is((dropInfo.internal_assets_data[0] as InternalNFTData).nft.token_ids.length, 1);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });

// test('Add a Ton of NFTs', async t => {
//     const {funder, keypomV3, nftContract, root} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nftAsset1 = {
//         nft_contract_id: nftContract.accountId
//     }

//     const dropId = "drop-id";
//     const assets_per_use = {
//         1: [nftAsset1]
//     }

//     let keyPairs = await generateKeyPairs(1);
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys,
//         },
//         attachedDeposit: NEAR.parse("10").toString()
//     })

//     // loop 50 times
//     for (let i = 0; i < 50; i++) {
//         await functionCall({
//             signer: nftContract,
//             receiver: keypomV3,
//             methodName: 'nft_on_transfer',
//             args: {
//                 sender_id: funder.accountId,
//                 token_id: `token${i}`,
//                 msg: dropId
//             }
//         })
//     }

//     let userBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
//     console.log('userBal: ', userBal)
//     t.assert(NEAR.parse(userBal).lt(NEAR.from("10")));

//     let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 1)

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 2);

//     let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log(`dropInfo: ${JSON.stringify(dropInfo)}`)
//     t.is(dropInfo.internal_assets_data.length, 1);
//     t.is((dropInfo.internal_assets_data[0] as InternalNFTData).nft.token_ids.length, 50);

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });