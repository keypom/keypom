import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNFTData, ExtNearData, InternalNFTData, InternalFTData } from "../utils/types";
import { formatNearAmount } from "near-api-js/lib/utils/format";
import { Near } from "@near-js/wallet-account";
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
    await funder.updateAccount({
        amount: NEAR.parse('20.2 N').toString()
    })
    const ali = await root.createSubAccount('ali');
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);

    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"} });

    

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
    t.context.accounts = { root, funder, keypomV3, ali };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// 1. Deposit can cover all
// 2. Balance can cover all
// 3. Deposit not enough, but with balance, it can be covered
// 3.5. Balance is not enough, but with deposit, it can be covered
// 4. Neither is enough

test('Paying all with Deposit', async t => {
    const {funder, keypomV3, root,
         ali} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    const nearAsset1: ExtNearData = {
        yoctonear: toYocto("6")
    }

    const asset_data_per_use = [
        {
            assets: [nearAsset1]
        },
    ]

    let preCreateBalance: {available: NEAR} = await funder.balance()
    // Expect Balance here to be about 10 NEAR
    t.is(parseInt(formatNearAmount(preCreateBalance.available.toString(), 1)) == 10, true)
    console.log(`Pre Bal: ${formatNearAmount(preCreateBalance.available.toString(), 5)}`)
    
    // This will nearly drain balance
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data: [{public_key: keyPairs.publicKeys[0]}]
            // public_keys: [keyPairs.publicKeys[0]]
        },
        attachedDeposit: NEAR.parse("10").toString(),
    })

    let postCreateBalance: {available: NEAR} = await funder.balance()
    console.log(`Post Bal: ${formatNearAmount(postCreateBalance.available.toString(), 5)}`)
    let amountUsed = BigInt(preCreateBalance.available.toString()) - BigInt(postCreateBalance.available.toString())

    // Amount used should be around 6 NEAR. Failure here would show an invalid/unsuccessful refund
    t.is(amountUsed > BigInt(NEAR.parse("6").toString()), true)
    t.is(amountUsed < BigInt(NEAR.parse("7").toString()), true)
});

test('Paying all with Balance', async t => {
    const {funder, keypomV3, root,
         ali} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    const nearAsset1: ExtNearData = {
        yoctonear: toYocto("6")
    }

    const asset_data_per_use = [
        {
            assets: [nearAsset1]
        },
    ]

    let preCreateNearBalance: {available: NEAR} = await funder.balance()

    let preCreateBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    console.log(`Pre Keypom Bal: ${preCreateBal}`)

    // This will nearly drain balance
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data: [{public_key: keyPairs.publicKeys[0]}]
            // public_keys: [keyPairs.publicKeys[0]]
        },
    })

    let postCreateNearBalance: {available: NEAR} = await funder.balance()
    let postCreateBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let refund = formatNearAmount((BigInt(preCreateBal) - BigInt(postCreateBal)).toString(), 5);

    // Ensure NEAR balance has not changed witin 2 dec places as Gas is estimated to be 0.001 NEAR for creating drop
    let nearBalChange = formatNearAmount((BigInt(preCreateNearBalance.available.toString()) - BigInt(postCreateNearBalance.available.toString())).toString(), 2)
    t.is(nearBalChange == "0", true)

    // Amount used should be around 6 NEAR. Failure here would show an invalid/unsuccessful refund  
    t.is(refund > "6", true)
    t.is(refund < "7", true)
});

test('Paying with Both', async t => {
    const {funder, keypomV3, root,
         ali} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    const nearAsset1: ExtNearData = {
        yoctonear: toYocto("15")
    }

    const asset_data_per_use = [
        {
            assets: [nearAsset1]
        },
    ]

    // Pre drop creation near and keypom balances
    let preCreateNearBalance: {available: NEAR} = await funder.balance()
    let preCreateBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    
    // Expect Balance here to be about 10 NEAR
    t.is(parseInt(formatNearAmount(preCreateNearBalance.available.toString(), 1)) == 10, true)
    
    // This will nearly drain balance
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data: [{public_key: keyPairs.publicKeys[0]}]
            // public_keys: [keyPairs.publicKeys[0]]
        },
        attachedDeposit: NEAR.parse("10").toString(),
    })

    // get near and keypom balances after drop creation
    let postCreateNearBalance: {available: NEAR} = await funder.balance()
    let postCreateBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

    // get changes in both, rounded to nearest 0.1 just as a rough estimate and eliminate factors such as gas
    let balanceChange = formatNearAmount((BigInt(preCreateBal) - BigInt(postCreateBal)).toString(), 1);
    let NEARamountUsed = formatNearAmount((BigInt(preCreateNearBalance.available.toString()) - BigInt(postCreateNearBalance.available.toString())).toString(), 1)

    // Amount used should be around 10 NEAR
    console.log(NEARamountUsed)
    t.is(NEARamountUsed == "10", true)
    // Keypom Balance should make up for rest, roughly 5 NEAR
    console.log(balanceChange)
    t.is(balanceChange == "5", true)
});




