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
    const ftContract1 = await root.createSubAccount('ft_contract_1');
    
    await keypomV3.deploy(`./out/mapping.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    await ftContract1.deploy(`./out/ft.wasm`);
    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId });
    await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });
    
    // Deposit storage
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})

    // Send FTs
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
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

interface FTAsset {
    contract_id: string;
    registration_cost: string;
    amount: string;
}

test('Single Null Claim', async t => {
    const {funder, ftContract1, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftAsset1: FTAsset = {
        contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
        amount: NEAR.parse("1").toString()
    }

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

    try {
        const keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPk});
        console.log('keyInfo (Should have panicked): ', keyInfo)
        t.fail('Key should have been deleted');
    } catch (e) {
        t.pass();
    }

    let endingDropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo (after drop is deleted): ', dropInfo)
    t.is(endingDropInfo, null);
    
    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});