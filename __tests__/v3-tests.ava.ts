import anyTest, { TestFn } from "ava";
import { claimTrialAccountDrop, createDrop, createTrialAccountDrop, getDrops, getUserBalance, parseNearAmount, trialCallMethod } from "keypom-js";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, displayBalances, generateKeyPairs, initKeypomConnection } from "./utils/general";
import { oneGtNear, sendFTs, totalSupply } from "./utils/ft-utils";
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
    
    await root.deploy(`./out/linkdrop.wasm`);
    await keypomV3.deploy(`./out/mapping.wasm`);
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', {});
    // Test users
    const minter = await root.createSubAccount('minter');

    const ftContract = await root.createSubAccount('ft_contract');
    await ftContract.deploy(`./out/ft.wasm`);
    await ftContract.call(ftContract, 'new_default_meta', { owner_id: ftContract, total_supply: totalSupply.toString() });
    
    // Mint the FTs
    await ftContract.call(ftContract, 'storage_deposit', { account_id: minter.accountId }, { attachedDeposit: NEAR.parse("1").toString() });
    await ftContract.call(ftContract, 'storage_deposit', { account_id: keypomV3.accountId }, { attachedDeposit: NEAR.parse("1").toString() });
    await ftContract.call(ftContract, 'ft_transfer', { receiver_id: minter.accountId, amount: NEAR.parse("1000").toString() }, { attachedDeposit: "1" });
        

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, minter, keypomV3, ftContract };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

//testing drop empty initialization and that default values perform as expected
// test('Send FTs', async t => {
//     const {minter, ftContract, keypomV3} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     await keypomV3.call(keypomV3, 'init_ft_data', {ft_contract_id: ftContract.accountId, registration_cost: NEAR.parse("0.0125")});
//     let res = await keypomV3.view('get_contract_data', {data_id: 0});
//     console.log('res after init ft: ', res)

//     let keypomFTBal = await ftContract.view('ft_balance_of', {account_id: keypomV3.accountId});
//     console.log('keypomFTBal before send: ', keypomFTBal)

//     await sendFTs(minter, NEAR.parse("10").toString(), keypomV3, ftContract, "0");

//     res = await keypomV3.view('get_contract_data', {data_id: 0});
//     console.log('res after send FTs: ', res)

//     keypomFTBal = await ftContract.view('ft_balance_of', {account_id: keypomV3.accountId});
//     console.log('keypomFTBal after send: ', keypomFTBal)

//     let minterFtBal = await ftContract.view('ft_balance_of', {account_id: minter.accountId});
//     console.log('minterFtBal before claim: ', minterFtBal)

//     await keypomV3.call(keypomV3, 'claim_ft_data', {data_id: 0, receiver_id: minter.accountId, amount: NEAR.parse("10").toString()}, {gas: "300000000000000"});

//     keypomFTBal = await ftContract.view('ft_balance_of', {account_id: keypomV3.accountId});
//     console.log('keypomFTBal after claim: ', keypomFTBal)

//     minterFtBal = await ftContract.view('ft_balance_of', {account_id: minter.accountId});
//     console.log('minterFtBal after claim: ', minterFtBal)

//     res = await keypomV3.view('get_contract_data', {data_id: 0});
//     console.log('res after claim: ', res)

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });

interface FTAsset {
    contract_id: string;
    registration_cost: string;
    amount: string;
}

test('Send FTs', async t => {
    const {minter, ftContract, keypomV3} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const ftContractData = {
        contract_id: ftContract.accountId,
        registration_cost: NEAR.parse("0.0125").toString(),
    }
    const ftAsset1: FTAsset = {
        ...ftContractData,
        amount: NEAR.parse("1").toString()
    }
    const ftAsset2: FTAsset = {
        ...ftContractData,
        amount: NEAR.parse("2").toString()
    }

    const dropId = "foobar123";
    const assets_per_use = {
        1: [ftAsset1],
    }
    let keyPairs = await generateKeyPairs(2);
    await keypomV3.call(keypomV3, 'create_drop', {drop_id: dropId, assets_per_use, public_keys: keyPairs.publicKeys}, {attachedDeposit: NEAR.parse("1").toString()});
    let dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo: ', dropInfo)

    await sendFTs(minter, NEAR.parse("100").toString(), keypomV3, ftContract, dropId);
    
    dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log('dropInfo after: ', dropInfo)

    let keyPk = keyPairs.publicKeys[0];
    const keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPk});
    console.log('keyInfo: ', keyInfo)
    
    await keypomV3.setKey(keyPairs.keys[0]);
    await keypomV3.call(keypomV3, 'claim', {receiver_id: minter.accountId}, {gas: keyInfo.required_gas});

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});