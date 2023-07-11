import anyTest, { TestFn } from "ava";
import { claimTrialAccountDrop, createDrop, createTrialAccountDrop, getDrops, getUserBalance, parseNearAmount, trialCallMethod } from "keypom-js";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, displayBalances, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtKeyInfo, InternalFTData } from "../utils/types";
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
    
    await keypomV3.deploy(`./out/mapping.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);

    await keypomV3.call(keypomV3, 'new', {root_account: root.accountId});
    await root.call(root, 'new', {});

    // Test users
    const funder = await root.createSubAccount('funder');

    const ftContract1 = await root.createSubAccount('ft_contract_1');
    const ftContract2 = await root.createSubAccount('ft_contract_2');
    const ftContract3 = await root.createSubAccount('ft_contract_3');
    
    await ftContract1.deploy(`./out/ft.wasm`);
    await ftContract2.deploy(`./out/ft.wasm`);
    await ftContract3.deploy(`./out/ft.wasm`);
    
    await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });
    await ftContract2.call(ftContract2, 'new_default_meta', { owner_id: ftContract2, total_supply: totalSupply.toString() });
    await ftContract3.call(ftContract3, 'new_default_meta', { owner_id: ftContract3, total_supply: totalSupply.toString() });
    
    // Deposit storage
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})

    await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})

    await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})

    // Send FTs
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, keypomV3, ftContract1, ftContract2, ftContract3 };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Failed Multi Claim', async t => {
    const {funder, ftContract1, ftContract2, ftContract3, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0").toString(),
        ft_amount: "1"
    }
    const ftAsset2: ExtFTData = {
        ft_contract_id: "bar",
        registration_cost: NEAR.parse("0.0125").toString(),
        ft_amount: NEAR.parse("0").toString()
    }
    const ftAsset3: ExtFTData = {
        ft_contract_id: "baz",
        registration_cost: NEAR.parse("0.0125").toString(),
        ft_amount: NEAR.parse("0").toString()
    }

    const dropId = "failed-multiclaim";
    const assets_per_use = {
        1: [ftAsset1, ftAsset2, ftAsset3],
        2: [ftAsset1],
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

    await sendFTs(funder, "1", keypomV3, ftContract1, dropId);

    let keypomFTBal = await ftContract1.view('ft_balance_of', {account_id: keypomV3.accountId});
    console.log('keypomFTBal: ', keypomFTBal)
    t.is(keypomFTBal, '1')
    
    let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo after: ', dropInfo)

    for (var asset of dropInfo.internal_assets_data) {
        let ftAsset = asset as InternalFTData;
        
        if (ftAsset.ft.contract_id === ftContract1.accountId) {
            t.is(ftAsset.ft.balance_avail, '1');
        } else {
            t.is(ftAsset.ft.balance_avail, '0');
        }
    }

    await keypomV3.setKey(keyPairs.keys[0]);
    let newAccountId = `new-account.${root.accountId}`;
    let keyPk = keyPairs.publicKeys[0];
    let keyInfo: ExtKeyInfo = await keypomV3.view('get_key_information', {key: keyPk});
    console.log('keyInfo: ', keyInfo)
    t.is(keyInfo.uses_remaining, 2);

    let response = await functionCall({
        signer: keypomV3,
        receiver: keypomV3,
        methodName: 'create_account_and_claim',
        args: {new_account_id: newAccountId, new_public_key: keyPk},
        //args: {account_id: root.accountId},
        gas: keyInfo.required_gas,
        shouldPanic: true
    })
    console.log('response: ', response)
    t.is(response, "false");
    
    dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log(`dropInfo: ${JSON.stringify(dropInfo)}`)
    for (var asset of dropInfo.internal_assets_data) {
        let ftAsset = asset as InternalFTData;
        
        if (ftAsset.ft.contract_id === ftContract1.accountId) {
            t.is(ftAsset.ft.balance_avail, '1');
        } else {
            t.is(ftAsset.ft.balance_avail, '0');
        }
    }


    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 2);

    keyInfo = await keypomV3.view('get_key_information', {key: keyPk});
    console.log('keyInfo: ', keyInfo)
    t.is(keyInfo.uses_remaining, 1);

    response = await functionCall({
        signer: keypomV3,
        receiver: keypomV3,
        methodName: 'claim',
        //args: {new_account_id: newAccountId, new_public_key: keyPk},
        args: {account_id: root.accountId},
        gas: keyInfo.required_gas,
        shouldPanic: true
    })
    console.log('response: ', response)
    t.is(response, "false");

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 0)

    try {
        keyInfo = await keypomV3.view('get_key_information', {key: keyPk});
        console.log('keyInfo: ', keyInfo)
        t.fail("Key should have been deleted");
    } catch(e) {
        t.pass();
    }

    // Drop should not be deleted since there's 1 FT still in it
    let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)
    t.assert(endingDropInfo != null);
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

// test('Overpay, Withdraw, Delete', async t => {
//     const {funder, ftContract1, ftContract2, ftContract3, keypomV3} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const ftAsset1: FTAsset = {
//         contract_id: ftContract1.accountId,
//         registration_cost: NEAR.parse("0.0125").toString(),
//         amount: NEAR.parse("1").toString()
//     }
//     const ftAsset2: FTAsset = {
//         contract_id: ftContract2.accountId,
//         registration_cost: NEAR.parse("0.0125").toString(),
//         amount: NEAR.parse("2").toString()
//     }
//     const ftAsset3: FTAsset = {
//         contract_id: ftContract3.accountId,
//         registration_cost: NEAR.parse("0.0125").toString(),
//         amount: NEAR.parse("3").toString()
//     }

//     const dropId = "underpay, withdraw, delete";
//     const assets_per_use = {
//         1: [ftAsset1, ftAsset2],
//         2: [ftAsset2, ftAsset3],
//         3: [ftAsset1, ftAsset3]
//     }
//     let keyPairs = await generateKeyPairs(50);
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
//     let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo: ', dropInfo)
//     for (var asset of dropInfo.internal_assets_data) {
//         let ftAsset = asset as InternalFTData;

//         t.is(ftAsset.ft.balance_avail, '0');
//     }

//     let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 50)

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 51);

//     await sendFTs(funder, NEAR.parse("1000").toString(), keypomV3, ftContract1, dropId);

//     let keypomFTBal = await ftContract1.view('ft_balance_of', {account_id: keypomV3.accountId});
//     console.log('keypomFTBal: ', keypomFTBal)
//     t.is(keypomFTBal, NEAR.parse("1000").toString());
    
//     dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo after: ', dropInfo)
//     for (var asset of dropInfo.internal_assets_data) {
//         let ftAsset = asset as InternalFTData;

//         if (ftAsset.ft.contract_id === ftContract1.accountId) {
//             t.is(ftAsset.ft.balance_avail, NEAR.parse("1000").toString());
//         } else {
//             t.is(ftAsset.ft.balance_avail, '0');
//         }
//     }

//     try {
//         await functionCall({
//             signer: funder,
//             receiver: keypomV3,
//             methodName: 'delete_keys',
//             args: {drop_id: dropId},
//             gas: LARGE_GAS,
//             attachedDeposit: "0"
//         })
//         t.fail('Delete keys should have failed since not all assets are withdrawn')
//     } catch(e) {
//         t.pass();
//     }

//     dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo after failed deletion: ', dropInfo)
//     for (var asset of dropInfo.internal_assets_data) {
//         let ftAsset = asset as InternalFTData;

//         if (ftAsset.ft.contract_id === ftContract1.accountId) {
//             t.is(ftAsset.ft.balance_avail, NEAR.parse("1000").toString());
//         } else {
//             t.is(ftAsset.ft.balance_avail, '0');
//         }
//     }

//     keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 50)

//     keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 51);

//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'withdraw_ft_balance',
//         args: {
//             drop_id: dropId, 
//             ft_contract_id: ftContract1.accountId, 
//             tokens_to_withdraw: NEAR.parse("1000").toString()
//         },
//         gas: LARGE_GAS,
//         attachedDeposit: "0"
//     })

//     keypomFTBal = await ftContract1.view('ft_balance_of', {account_id: keypomV3.accountId});
//     console.log('keypomFTBal: ', keypomFTBal)
//     t.is(keypomFTBal, '0')

//     let funderFTBal = await ftContract1.view('ft_balance_of', {account_id: funder.accountId});
//     console.log('funderFTBal: ', funderFTBal)
//     t.is(funderFTBal, NEAR.parse("1000").toString());

//     dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo after failed deletion: ', dropInfo)
//     for (var asset of dropInfo.internal_assets_data) {
//         let ftAsset = asset as InternalFTData;
        
//         t.is(ftAsset.ft.balance_avail, '0');
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'delete_keys',
//         args: {drop_id: dropId},
//         gas: LARGE_GAS,
//         attachedDeposit: "0"
//     })

//     try {
//         keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//         console.log('keysForDrop: ', keysForDrop)
//         t.fail('Drop should have been deleted so method should panic')
//     } catch (e) {
//         t.pass();
//     }

//     keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

//     // After keys have been deleted, 50 * 6 * 0.0125 = 3.75 should be returned to the funder
//     let userBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
//     console.log('userBal: ', userBal)
//     t.assert(NEAR.from(userBal).gte(NEAR.parse("3.75")))

//     let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo (after drop is deleted): ', dropInfo)
//     t.is(endingDropInfo, null);
    
//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });

// test('Create & Delete Empty Drop', async t => {
//     const {funder, ftContract1, ftContract2, ftContract3, keypomV3} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const ftAsset1: FTAsset = {
//         contract_id: ftContract1.accountId,
//         registration_cost: NEAR.parse("0.0125").toString(),
//         amount: NEAR.parse("1").toString()
//     }
//     const ftAsset2: FTAsset = {
//         contract_id: ftContract2.accountId,
//         registration_cost: NEAR.parse("0.0125").toString(),
//         amount: NEAR.parse("2").toString()
//     }
//     const ftAsset3: FTAsset = {
//         contract_id: ftContract3.accountId,
//         registration_cost: NEAR.parse("0.0125").toString(),
//         amount: NEAR.parse("3").toString()
//     }

//     const dropId = "overpay, withdraw, delete";
//     const assets_per_use = {
//         1: [ftAsset1, ftAsset2],
//         2: [ftAsset2, ftAsset3],
//         3: [ftAsset1, ftAsset3]
//     }
//     let keyPairs = await generateKeyPairs(0);
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
//     let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo: ', dropInfo)
//     for (var asset of dropInfo.internal_assets_data) {
//         let ftAsset = asset as InternalFTData;

//         t.is(ftAsset.ft.balance_avail, '0');
//     }

//     let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 0)

//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

//     await sendFTs(funder, NEAR.parse("1000").toString(), keypomV3, ftContract1, dropId);

//     let keypomFTBal = await ftContract1.view('ft_balance_of', {account_id: keypomV3.accountId});
//     console.log('keypomFTBal: ', keypomFTBal)
//     t.is(keypomFTBal, NEAR.parse("1000").toString());
    
//     dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo after: ', dropInfo)
//     for (var asset of dropInfo.internal_assets_data) {
//         let ftAsset = asset as InternalFTData;

//         if (ftAsset.ft.contract_id === ftContract1.accountId) {
//             t.is(ftAsset.ft.balance_avail, NEAR.parse("1000").toString());
//         } else {
//             t.is(ftAsset.ft.balance_avail, '0');
//         }
//     }

//     try {
//         await functionCall({
//             signer: funder,
//             receiver: keypomV3,
//             methodName: 'delete_keys',
//             args: {drop_id: dropId},
//             gas: LARGE_GAS,
//             attachedDeposit: "0"
//         })
//         t.fail('Delete keys should have failed since not all assets are withdrawn')
//     } catch(e) {
//         t.pass();
//     }

//     dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo after failed deletion: ', dropInfo)
//     for (var asset of dropInfo.internal_assets_data) {
//         let ftAsset = asset as InternalFTData;

//         if (ftAsset.ft.contract_id === ftContract1.accountId) {
//             t.is(ftAsset.ft.balance_avail, NEAR.parse("1000").toString());
//         } else {
//             t.is(ftAsset.ft.balance_avail, '0');
//         }
//     }

//     keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//     console.log('keysForDrop: ', keysForDrop)
//     t.is(keysForDrop, 0)

//     keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'withdraw_ft_balance',
//         args: {
//             drop_id: dropId, 
//             ft_contract_id: ftContract1.accountId, 
//             tokens_to_withdraw: NEAR.parse("1000").toString()
//         },
//         gas: LARGE_GAS,
//         attachedDeposit: "0"
//     })

//     keypomFTBal = await ftContract1.view('ft_balance_of', {account_id: keypomV3.accountId});
//     console.log('keypomFTBal: ', keypomFTBal)
//     t.is(keypomFTBal, '0')

//     let funderFTBal = await ftContract1.view('ft_balance_of', {account_id: funder.accountId});
//     console.log('funderFTBal: ', funderFTBal)
//     t.is(funderFTBal, NEAR.parse("1000").toString());

//     dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo after failed deletion: ', dropInfo)
//     for (var asset of dropInfo.internal_assets_data) {
//         let ftAsset = asset as InternalFTData;
        
//         t.is(ftAsset.ft.balance_avail, '0');
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'delete_keys',
//         args: {drop_id: dropId},
//         gas: LARGE_GAS,
//         attachedDeposit: "0"
//     })

//     try {
//         keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
//         console.log('keysForDrop: ', keysForDrop)
//         t.fail('Drop should have been deleted so method should panic')
//     } catch (e) {
//         t.pass();
//     }

//     keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

//     let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
//     console.log('dropInfo (after drop is deleted): ', dropInfo)
//     t.is(endingDropInfo, null);
    
//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });
