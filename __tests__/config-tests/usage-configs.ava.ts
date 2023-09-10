import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist, assertNFTKeyData, assertFTBalance } from "../utils/general";
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

    const keypom_args_linkdrop = await root.createSubAccount(`keypom-args-linkdrop`)
    await keypom_args_linkdrop.deploy(`./__tests__/ext-wasm/create-account-keypom-injected-args-linkdrop.wasm`)
    
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
    await keypom_args_linkdrop.call(keypom_args_linkdrop, 'new', {})
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
    t.context.accounts = { root, funder, ftContract1, keypomV3, ali, keypom_args_linkdrop };

    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Usage Config
//    Different permissions for keys
//    Auto delete drop when empty
//    Account creation fields being properly sent


test('Different Permission for Keys + Auto Delete when Empty', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("0.2").toString()
    }

    const asset_data_per_use = [
        {
            assets: [nearAsset1],
            uses: 1,
            config: {
                permissions: "claim"
            }
        },
        {
            assets: [nearAsset1],
            uses: 1,
            config: {
                permissions: "create_account_and_claim"
            }
        },
        {
            assets: [nearAsset1],
            uses: 2
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

    // ******************* Claiming *******************
    let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining == 4, true)
    // First claim is limited to claim, try CAAC
    try{
        await claimWithRequiredGas({
            keypom: keypomV3,
            keyPair: keyPairs.keys[0],
            root,
            createAccount: true
        })
        t.fail()
    }catch{
        await claimWithRequiredGas({
            keypom: keypomV3,
            keyPair: keyPairs.keys[0],
            root,
            receiverId: ali.accountId
        })
    }

    keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining == 3, true)

    // Second Claim is limited to CAAC, try claim
    try{
        await claimWithRequiredGas({
            keypom: keypomV3,
            keyPair: keyPairs.keys[0],
            root,
            receiverId: ali.accountId
        })
        t.fail()
    }catch{
        await claimWithRequiredGas({
            keypom: keypomV3,
            keyPair: keyPairs.keys[0],
            root,
            createAccount: true
        })
    }

    keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining == 2, true)

    // Next Two Claims not limited
    // Claim
    try{
        await claimWithRequiredGas({
            keypom: keypomV3,
            keyPair: keyPairs.keys[0],
            root,
            receiverId: ali.accountId
        })
    }catch{
        t.fail()
    }

    keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining == 1, true)

    //CAAC
    try{
        await claimWithRequiredGas({
            keypom: keypomV3,
            keyPair: keyPairs.keys[0],
            root,
            createAccount: true
        })
    }catch{
        t.fail()
    }

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
    let finalBal = await keypomV3.balance();
    t.deepEqual(initialBal.stateStaked, finalBal.stateStaked)
});

test('Account creation fields being properly sent', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, keypom_args_linkdrop} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("0.2").toString()
    }

    const asset_data_per_use = [
        {
            assets: [nearAsset1],
            uses: 1,
            config: {
                account_creation_keypom_args: {
                    drop_id_field: "keypom_drop_id"
                },
                root_account_id: keypom_args_linkdrop.accountId
            }
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

    // ******************* Claiming *******************
    let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining == 1, true)

    //CAAC
    try{
        await claimWithRequiredGas({
            keypom: keypomV3,
            keyPair: keyPairs.keys[0],
            root,
            createAccount: true,
            receiverId: "abcdefghijklmnopqrstuvwxyz1234567.keypom-args-linkdrop.test.near"
        })
    }catch{
        t.fail()
    }

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
    let finalBal = await keypomV3.balance();
    t.deepEqual(initialBal.stateStaked, finalBal.stateStaked)
});
