import anyTest, { TestFn } from "ava";
import { claimTrialAccountDrop, createDrop, createTrialAccountDrop, getDrops, getUserBalance, parseNearAmount, trialCallMethod } from "keypom-js";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, displayBalances, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
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
    await keypomV3.call(keypomV3, 'new', {});
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
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString()})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString()})

    await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString()})
    await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString()})

    await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString()})
    await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString()})

    // Send FTs
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1"})
    await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1"})
    await functionCall({signer: ftContract3, receiver: ftContract3, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1"})

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

interface FTAsset {
    contract_id: string;
    registration_cost: string;
    amount: string;
}

test('Single Use Single FT', async t => {
    const {funder, ftContract1, keypomV3} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftContractData = {
        contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
    }
    const ftAsset: FTAsset = {
        ...ftContractData,
        amount: NEAR.parse("1").toString()
    }

    const dropId = "underpay-delete-all";
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
    let dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)

    await sendFTs(funder, NEAR.parse("100").toString(), keypomV3, ftContract1, dropId);
    
    dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo after: ', dropInfo)

    // let keyPk = keyPairs.publicKeys[0];
    // const keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPk});
    // console.log('keyInfo: ', keyInfo)

    // // await functionCall({
    // //     signer: funder,
    // //     receiver: keypomV3,
    // //     methodName: 'withdraw_ft_balance',
    // //     args: {
    // //         drop_id: dropId, 
    // //         ft_contract_id: ftContract.accountId, 
    // //         tokens_to_withdraw: NEAR.parse("1").toString()
    // //     },
    // //     gas: LARGE_GAS,
    // //     attachedDeposit: "0"
    // // })

    // await functionCall({
    //     signer: funder,
    //     receiver: keypomV3,
    //     methodName: 'delete_keys',
    //     args: {drop_id: dropId},
    //     gas: LARGE_GAS,
    //     attachedDeposit: "0"
    // })

    // await functionCall({
    //     signer: funder,
    //     receiver: keypomV3,
    //     methodName: 'delete_drop',
    //     args: {drop_id: dropId},
    //     gas: LARGE_GAS,
    //     attachedDeposit: "0",
    // })

    // await functionCall({
    //     signer: funder,
    //     receiver: keypomV3,
    //     methodName: 'withdraw_ft_balance',
    //     args: {
    //         drop_id: dropId, 
    //         ft_contract_id: ftContract.accountId, 
    //         tokens_to_withdraw: NEAR.parse("99").toString()
    //     },
    //     gas: LARGE_GAS,
    //     attachedDeposit: "0"
    // })

    // await functionCall({
    //     signer: funder,
    //     receiver: keypomV3,
    //     methodName: 'delete_drop',
    //     args: {drop_id: dropId},
    //     gas: LARGE_GAS,
    //     attachedDeposit: "0"
    // })
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

/*
    TODO TESTS:

    For each of the following:
    - Single Use Single FT
    - Single Use Multi-FT
    - Multi-Use Single FT
    - Multi-Use Multi-FT
    
    1. Underpay, delete all at once
    2. Overpay, delete all at once (gas issues most probably)
    3. Overpay, withdraw, delete
*/