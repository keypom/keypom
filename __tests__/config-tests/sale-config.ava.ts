import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist, assertNFTKeyData, assertFTBalance, delay } from "../utils/general";
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

    const bob = await root.createSubAccount('bob');
    await bob.updateAccount({
        amount: NEAR.parse('10000000 N').toString()
    })
    
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
    await functionCall({
        signer: ali,
        receiver: keypomV3,
        methodName: 'add_to_balance',
        args: {},
        attachedDeposit: NEAR.parse("10").toString(),
        shouldLog: false
    })
    await functionCall({
        signer: bob,
        receiver: keypomV3,
        methodName: 'add_to_balance',
        args: {},
        attachedDeposit: NEAR.parse("10").toString(),
        shouldLog: false
    })
    
    // Save state for test runs
    t.context.worker = worker;
    // t.context.accounts = { root, funder, keypomV3, ali };
    t.context.accounts = { root, funder, keypomV3, ali, bob };

    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Time Config
//     Start timestamp
//     End Timestamp
//     Throttle timestamp
//     Interval (both with & without start specified)



test('User in Add Keys Allowlist', async t => {
    const {funder, keypomV3, root, ali, bob} = t.context.accounts;
    const ONE_SECOND_NS = 1e9
    const ONE_SECOND_MS = 1e3
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 4;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const asset_data = [
        {
            assets: [null],
            uses: 1,
        },
    ]

    // Ali on list, bob is not
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
            }],
            drop_config: {
                add_key_allowlist: [ali.accountId]
            }
        },
    }) 
    let keysForDrop: number = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    t.is(keysForDrop, 1)
    
    //Funder can add
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data: [{
                public_key: keyPairs.publicKeys[1],
            }],
        },
        attachedDeposit: NEAR.parse("1").toString(),
    })

    keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    t.is(keysForDrop, 2)

    // Ali can add
    await functionCall({
        signer: ali,
        receiver: keypomV3,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data: [{
                public_key: keyPairs.publicKeys[2],
            }], 
        },
        attachedDeposit: NEAR.parse("1").toString(),
    })

    keysForDrop= await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    t.is(keysForDrop, 3)

    // Bob cannot add
    try{
        await functionCall({
            signer: bob,
            receiver: keypomV3,
            methodName: 'add_keys',
            args: {
                drop_id: dropId,
                key_data: [{
                    public_key: keyPairs.publicKeys[3],
                }],
            },
            attachedDeposit: NEAR.parse("1").toString(),
        })
    }catch(e){
        keysForDrop= await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
        t.is(keysForDrop, 3)
    }

    // last key will not exist
    for(let i = 0; i < keyPairs.publicKeys.length - 1; i++){
        t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[i]), true)
    }

    t.is(await doesDropExist(keypomV3, dropId), true)
    
});
