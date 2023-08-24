import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist, assertNFTKeyData } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNFTData, InternalNFTData, InternalFTData, ExtNearData, TokenMetadata } from "../utils/types";
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
        amount: NEAR.parse('10000000 N').toString()
    })
    const ali = await root.createSubAccount('ali');
    await ali.updateAccount({
        amount: NEAR.parse('10000000 N').toString()
    })
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);

    const ftContract1 = await root.createSubAccount('ft_contract_1');
    
    await ftContract1.deploy(`./__tests__/ext-wasm/ft.wasm`);
    
    await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });
     
    // Deposit storage
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // Send FTs
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    
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
    await functionCall({
        signer: ali,
        receiver: keypomV3,
        methodName: 'add_to_balance',
        args: {},
        attachedDeposit: NEAR.parse("10").toString(),
        shouldLog: false
    })
    
    // Save state for test runs
    t.context.worker = worker;
    // t.context.accounts = { root, funder, keypomV3, ali };
    t.context.accounts = { root, funder, keypomV3, ftContract1, ali };

    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// ************** This covers case of asset ID not being present **************
test('Tokens to a non-FT drop', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = [
        {
            assets: [nearAsset1],
            uses: 1,
        },
    ]

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
            }],
        },
    }) 

    // ******************* Adding Assets *******************
    let preCreateBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

    try{
        await sendFTs(funder, "5", keypomV3, ftContract1, dropId)
    }catch(e){
        console.log(e)
    }
    
    let postCreateBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let balChange = formatNearAmount((BigInt(preCreateBal) - BigInt(postCreateBal)).toString(), 5);
    console.log(balChange)
    // No asset transfer thus no balance change
    t.is(balChange == "0", true)

    // Token should NOT show up in expected assets
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [],
    })
});

// ************** This covers case of sending assets to not your drop **************
test('Tokens to the Wrong Drop - Others with Same Assets', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, nftContract2, ali} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const wrongDropId = "wrong-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost:NEAR.parse("1").toString(),
        ft_amount: NEAR.parse("5").toString(), 
    }

    const asset_data_per_use = [
        {
            assets: [ftAsset1],
            uses: 1
        }
    ]

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: wrongDropId,
            asset_data: asset_data_per_use,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
            }],
        },
    }) 

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId: wrongDropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: "0"
        }]
    })

    // ******************* Adding Assets *******************
    let preSendBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

    try{
       await sendFTs(funder, "5", keypomV3, ftContract1, wrongDropId)
    }catch(e){
        console.log(e)
    }

    let postSendBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let balChange = formatNearAmount((BigInt(preSendBal) - BigInt(postSendBal)).toString(), 5);
    console.log(balChange)
    t.is(balChange == "0", true)

    // Should succeed - ANYBODY can fund your drops...
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId: wrongDropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: "5"
        }]
    })
});




