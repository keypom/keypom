import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Provider, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertFTBalance, assertKeypomInternalAssets, displayBalances, doesDropExist, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
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
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await keypomV3.call(keypomV3, 'new', {root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"}});
    // Test users
    const funder = await root.createSubAccount('funder');

    const ftContract1 = await root.createSubAccount('ft_contract_1');
    // const ftContract2 = await root.createSubAccount('ft_contract_2');
    // const ftContract3 = await root.createSubAccount('ft_contract_3');
    
    await ftContract1.deploy(`./__tests__/ext-wasm/ft.wasm`);
    // await ftContract2.deploy(`./__tests__/ext-wasm/ft.wasm`);
    // await ftContract3.deploy(`./__tests__/ext-wasm/ft.wasm`);
    
    await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });
    // await ftContract2.call(ftContract2, 'new_default_meta', { owner_id: ftContract2, total_supply: totalSupply.toString() });
    // await ftContract3.call(ftContract3, 'new_default_meta', { owner_id: ftContract3, total_supply: totalSupply.toString() });
    
    // Deposit storage
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})

    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})

    // await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})

    // Send FTs
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    // await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, keypomV3, ftContract1 };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// ******************** Underfunded Drop cannot be deleted until all is withdraw ********************
test('Underpay, Withdraw, Delete', async t => {
    const {funder, ftContract1, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftAsset: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
        ft_amount: NEAR.parse("1").toString()
    }

    const dropId = "underpay, withdraw, delete";
    const asset_data = [{
        assets: [ftAsset],
        uses: 1
    }]

    let numKeys = 50
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
        attachedDeposit: NEAR.parse("10").toString(),
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

    // Drop should be deleted since all keys and assets are gone
    t.is(await doesDropExist(keypomV3, dropId),false)

    // After keys have been deleted, 50 * 0.0125 = 0.625 should be returned to the funder
    let userBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
    console.log('userBal: ', userBal)
    t.assert(NEAR.from(userBal).gte(NEAR.parse("0.625")))
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

// ******************** Overfunded Drop cannot be deleted until all is withdraw ********************
test('Overpay, Withdraw, Delete', async t => {
    const {funder, ftContract1, keypomV3} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftAsset: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
        ft_amount: NEAR.parse("1").toString()
    }

    const dropId = "overpay, withdraw, delete";
    const asset_data = [{
        assets: [ftAsset],
        uses: 1
    }]

    let numKeys = 50
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

    t.is(await doesDropExist(keypomV3, dropId), false)

    // After keys have been deleted, 50 * 0.0125 = 0.625 should be returned to the funder
    let userBal: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
    console.log('userBal: ', userBal)
    t.assert(NEAR.from(userBal).gte(NEAR.parse("0.625")))
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

// ******************** Empty Drop cannot be deleted until all is withdraw ********************
test('Create & Delete Empty Drop', async t => {
    const {funder, ftContract1, keypomV3} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftAsset: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
        ft_amount: NEAR.parse("1").toString()
    }

    const dropId = "create delete empty";
    const asset_data = [{
        assets: [ftAsset],
        uses: 1
    }]
    let keyPairs = await generateKeyPairs(0);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId, 
            asset_data, 
            key_data: []
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

    // Calling delete_keys on empty drop should delete drop
    t.is(await doesDropExist(keypomV3, dropId), false)
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});