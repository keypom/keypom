import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertFTBalance, assertKeypomInternalAssets, displayBalances, doesDropExist, doesKeyExist, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
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
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);

    await keypomV3.call(keypomV3, 'new', {root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"}});
    await root.call(root, 'new', {});

    // Test users
    const funder = await root.createSubAccount('funder');

    const ftContract1 = await root.createSubAccount('ft_contract_1');
    const ftContract2 = await root.createSubAccount('ft_contract_2');
    const ftContract3 = await root.createSubAccount('ft_contract_3');
    
    await ftContract1.deploy(`./__tests__/ext-wasm/ft.wasm`);
    await ftContract2.deploy(`./__tests__/ext-wasm/ft.wasm`);
    await ftContract3.deploy(`./__tests__/ext-wasm/ft.wasm`);
    
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

test('Successful Single and Multi Claim', async t => {
    const {funder, ftContract1, ftContract2, ftContract3, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    // Create drop and load assets
    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: "1"
    }
    const ftAsset2: ExtFTData = {
        ft_contract_id: ftContract2.accountId,
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: "1"
    }
    const ftAsset3: ExtFTData = {
        ft_contract_id: ftContract3.accountId,
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: "1"
    }

    const dropId = "successful-multiclaim";
    const asset_data = [
        {
            assets: [ftAsset1, ftAsset2, ftAsset3],
            uses: 1
        },
        {
            assets: [ftAsset1],
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
            key_data: [
                {
                    public_key: keyPairs.publicKeys[0]
                }
            ]
        },
        attachedDeposit: NEAR.parse("10").toString()
    })

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '0',
            },{
                contract_id: ftContract2.accountId,
                balance_avail: '0',
            },{
                contract_id: ftContract3.accountId,
                balance_avail: '0',
        }]
    })

    await sendFTs(funder, "2", keypomV3, ftContract1, dropId);
    await sendFTs(funder, "1", keypomV3, ftContract2, dropId);
    await sendFTs(funder, "1", keypomV3, ftContract3, dropId);

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: '2'
    });

    await assertFTBalance({
        ftContract: ftContract2,
        accountId: keypomV3.accountId,
        amountOwned: '1'
    });

    await assertFTBalance({
        ftContract: ftContract3,
        accountId: keypomV3.accountId,
        amountOwned: '1'
    });

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '2',
            },{
                contract_id: ftContract2.accountId,
                balance_avail: '1',
            },{
                contract_id: ftContract3.accountId,
                balance_avail: '1',
        }]
    })

    let keyInfo: ExtKeyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 2);

    // Claim drop
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    // Ensure Asset Transfer Succeeded
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '1',
            },{
                contract_id: ftContract2.accountId,
                balance_avail: '0',
            },{
                contract_id: ftContract3.accountId,
                balance_avail: '0',
        }]
    })

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: claimingAccount,
        amountOwned: '1'
    });

    await assertFTBalance({
        ftContract: ftContract2,
        accountId: claimingAccount,
        amountOwned: '1'
    });

    await assertFTBalance({
        ftContract: ftContract3,
        accountId: claimingAccount,
        amountOwned: '1'
    });

     // Claim drop
     result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")
    claimingAccount = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: claimingAccount,
        amountOwned: '1'
    });

    await assertFTBalance({
        ftContract: ftContract2,
        accountId: claimingAccount,
        amountOwned: '0'
    });

    await assertFTBalance({
        ftContract: ftContract3,
        accountId: claimingAccount,
        amountOwned: '0'
    });

    // Drop and Key should both be deleted as assets are fully depleted
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

// TEST THIS AFTER REFUND FIX!

// THIS FAILS SINCE THE CONTRACT DOESNT THINK FTS HAVE BEEN TRANSFERRED BUT THE PROMISE ACTUALLY COMPLETES
test('Failed Single and Multi Claim', async t => {
    const {funder, ftContract1, ftContract2, ftContract3, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    // Create drop and load assets
    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: "1"
    }
    const ftAsset2: ExtFTData = {
        ft_contract_id: "eh",
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: "0"
    }
    const ftAsset3: ExtFTData = {
        ft_contract_id: "bee",
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: "0"
    }

    const dropId = "failed-multiclaim";
    const asset_data = [
        {
            assets: [ftAsset1, ftAsset2, ftAsset3],
            uses: 1
        },
        {
            assets: [ftAsset1],
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
            key_data: [
                {
                    public_key: keyPairs.publicKeys[0]
                }
            ]
        },
        attachedDeposit: NEAR.parse("10").toString()
    })

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '0',
            },{
                contract_id: "eh",
                balance_avail: '0',
            },{
                contract_id: "bee",
                balance_avail: '0',
        }]
    })

    await sendFTs(funder, "1", keypomV3, ftContract1, dropId);
   

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: '1'
    });


    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '1',
            },{
                contract_id: "eh",
                balance_avail: '0',
            },{
                contract_id: "bee",
                balance_avail: '0',
        }]
    })

    let keyInfo: ExtKeyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 2);

    // Claim drop
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        shouldPanic: true
    })
    t.is(result.response, "false")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: claimingAccount,
        amountOwned: '1'
    });

    // Ensure Assets Stayed the Same
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '0',
            },{
                contract_id: "eh",
                balance_avail: '0',
            },{
                contract_id: "bee",
                balance_avail: '0',
        }]
    })

     // Claim drop
     result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        shouldPanic: false
    })
    t.is(result.response, "true")
    claimingAccount = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    // Drop has no more key uses or assets
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Failed Claim due to FT Balance', async t => {
    const {funder, ftContract1, ftContract2, ftContract3, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    // Create drop and load assets
    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: "2"
    }

    const dropId = "failed-multiclaim";
    const asset_data = [
        {
            assets: [ftAsset1],
            uses: 2
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
            key_data: [
                {
                    public_key: keyPairs.publicKeys[0]
                }
            ]
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

    await sendFTs(funder, "3", keypomV3, ftContract1, dropId);
   

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: keypomV3.accountId,
        amountOwned: '3'
    });


    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '3',
            }]
    })

    let keyInfo: ExtKeyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 2);

    // Claim drop
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    await assertFTBalance({
        ftContract: ftContract1,
        accountId: claimingAccount,
        amountOwned: '2'
    });

    // Ensure Assets Stayed the Same
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '1',
            }]
    })

     // Insufficient balance will just skip asset claim
     result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
    })
    // Create account should succeed
    t.is(result.response, "true")
    claimingAccount = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    // Drop should exist but key should be deleted
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)

    // Ensure Assets Stayed the Same
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
                contract_id: ftContract1.accountId,
                balance_avail: '1',
            }]
    })
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

