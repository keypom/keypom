import anyTest, { TestFn } from "ava";
import { claimTrialAccountDrop, createDrop, createTrialAccountDrop, getDrops, getUserBalance, parseNearAmount, trialCallMethod } from "keypom-js";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertFTBalance, assertKeypomInternalAssets, displayBalances, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, InternalFTData } from "../utils/types";
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
    await keypomV3.call(keypomV3, 'new', {root_account: root.accountId});
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

test('Underpay, Withdraw, Delete', async t => {
    const {funder, ftContract1, keypomV3} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftContractData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
    }
    const ftAsset: ExtFTData = {
        ...ftContractData,
        ft_amount: NEAR.parse("1").toString()
    }

    const dropId = "underpay, withdraw, delete";
    const assets_per_use = {
        1: [ftAsset],
    }
    let keyPairs = await generateKeyPairs(50);
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

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '0',
        }]
    })

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 50)
    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 51);

    await sendFTs(funder, "5", keypomV3, ftContract1, dropId);

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: '5'
    });
    
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '5',
        }]
    })

    try {
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: 'delete_keys',
            args: {drop_id: dropId},
            gas: LARGE_GAS,
            attachedDeposit: "0"
        })
        t.fail('Delete keys should have failed since not all assets are withdrawn')
    } catch(e) {
        t.pass();
    }

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '5',
        }]
    })

    keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 50)

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 51);

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'withdraw_ft_balance',
        args: {
            drop_id: dropId, 
            ft_contract_id: ftContract1.accountId, 
            tokens_to_withdraw: '5'
        },
        gas: LARGE_GAS,
        attachedDeposit: "0"
    })

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: '0'
    });

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: funder.accountId,
        amountOwned: NEAR.parse("1000").toString()
    });

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '0',
        }]
    })

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'delete_keys',
        args: {drop_id: dropId},
        gas: LARGE_GAS,
        attachedDeposit: "0"
    })

    try {
        keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
        console.log('keysForDrop: ', keysForDrop)
        t.fail('Drop should have been deleted so method should panic')
    } catch (e) {
        t.pass();
    }

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    // After keys have been deleted, 50 * 0.0125 = 0.625 should be returned to the funder
    let userBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
    console.log('userBal: ', userBal)
    t.assert(NEAR.from(userBal).gte(NEAR.parse("0.625")))

    let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo (after drop is deleted): ', endingDropInfo)
    t.is(endingDropInfo, null);
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Overpay, Withdraw, Delete', async t => {
    const {funder, ftContract1, keypomV3} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftContractData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
    }
    const ftAsset: ExtFTData = {
        ...ftContractData,
        ft_amount: NEAR.parse("1").toString()
    }

    const dropId = "overpay, withdraw, delete";
    const assets_per_use = {
        1: [ftAsset],
    }
    let keyPairs = await generateKeyPairs(50);
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
    
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '0',
        }]
    })

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 50)

    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 51);

    await sendFTs(funder, NEAR.parse("1000").toString(), keypomV3, ftContract1, dropId);

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: NEAR.parse("1000").toString()
    });
    
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: NEAR.parse("1000").toString(),
        }]
    })

    try {
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: 'delete_keys',
            args: {drop_id: dropId},
            gas: LARGE_GAS,
            attachedDeposit: "0"
        })
        t.fail('Delete keys should have failed since not all assets are withdrawn')
    } catch(e) {
        t.pass();
    }

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: NEAR.parse("1000").toString(),
        }]
    })

    keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 50)

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 51);

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'withdraw_ft_balance',
        args: {
            drop_id: dropId, 
            ft_contract_id: ftContract1.accountId, 
            tokens_to_withdraw: NEAR.parse("1000").toString()
        },
        gas: LARGE_GAS,
        attachedDeposit: "0"
    })

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: '0'
    });

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: funder.accountId,
        amountOwned: NEAR.parse("1000").toString()
    });

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '0',
        }]
    })

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'delete_keys',
        args: {drop_id: dropId},
        gas: LARGE_GAS,
        attachedDeposit: "0"
    })

    try {
        keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
        console.log('keysForDrop: ', keysForDrop)
        t.fail('Drop should have been deleted so method should panic')
    } catch (e) {
        t.pass();
    }

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    // After keys have been deleted, 50 * 0.0125 = 0.625 should be returned to the funder
    let userBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
    console.log('userBal: ', userBal)
    t.assert(NEAR.from(userBal).gte(NEAR.parse("0.625")))

    let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo (after drop is deleted): ', endingDropInfo)
    t.is(endingDropInfo, null);
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Create & Delete Empty Drop', async t => {
    const {funder, ftContract1, keypomV3} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftContractData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
    }
    const ftAsset: ExtFTData = {
        ...ftContractData,
        ft_amount: NEAR.parse("1").toString()
    }

    const dropId = "overpay, withdraw, delete";
    const assets_per_use = {
        1: [ftAsset],
    }
    let keyPairs = await generateKeyPairs(0);
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

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '0',
        }]
    })

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 0)

    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    await sendFTs(funder, NEAR.parse("1000").toString(), keypomV3, ftContract1, dropId);

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: NEAR.parse("1000").toString()
    });

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: NEAR.parse("1000").toString(),
        }]
    })

    try {
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: 'delete_keys',
            args: {drop_id: dropId},
            gas: LARGE_GAS,
            attachedDeposit: "0"
        })
        t.fail('Delete keys should have failed since not all assets are withdrawn')
    } catch(e) {
        t.pass();
    }

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: NEAR.parse("1000").toString(),
        }]
    })

    keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 0)

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'withdraw_ft_balance',
        args: {
            drop_id: dropId, 
            ft_contract_id: ftContract1.accountId, 
            tokens_to_withdraw: NEAR.parse("1000").toString()
        },
        gas: LARGE_GAS,
        attachedDeposit: "0"
    })

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: '0'
    });

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: funder.accountId,
        amountOwned: NEAR.parse("1000").toString()
    });

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '0',
        }]
    })


    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'delete_keys',
        args: {drop_id: dropId},
        gas: LARGE_GAS,
        attachedDeposit: "0"
    })

    try {
        keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
        console.log('keysForDrop: ', keysForDrop)
        t.fail('Drop should have been deleted so method should panic')
    } catch (e) {
        t.pass();
    }

    keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

    let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo (after drop is deleted): ', endingDropInfo)
    t.is(endingDropInfo, null);
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

// Add a test to ensure that if incorrect public keys are passed in and a panic occurs, keys aren't deleted
test('Panic In Deletion, Ensure Keys Are Not Deleted', async t => {
    const {funder, ftContract1, keypomV3} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftAsset: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
        ft_amount: NEAR.parse("1").toString()
    }

    const dropId = "deletion-test";
    const assets_per_use = {
        1: [ftAsset],
    }
    let keyPairs = await generateKeyPairs(40);
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

    // First 40 PKs correctly belong to drop but the last 1 does not 
    let pksToDelete = keyPairs.publicKeys.concat((await generateKeyPairs(1)).publicKeys);
    try {
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: 'delete_keys',
            args: {drop_id: dropId, public_keys: pksToDelete},
            gas: LARGE_GAS,
            attachedDeposit: "0"
        })
        t.fail('Delete keys should have panicked because key not belonging to drop was passed in')
    } catch(e) {
        t.pass();
    }

    // Check 
    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    //console.log('keypomKeys: ', keypomKeys)
    // There should still be 40 keys + 1 FAK
    t.is(keypomKeys.keys.length, 41);

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 40)
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});
