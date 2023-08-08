import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist, assertNFTKeyData, assertProperStorage } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNFTData, InternalNFTData, InternalFTData, ExtNearData, TokenMetadata, ExtKeyInfo } from "../utils/types";
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
    t.context.accounts = { root, funder, keypomV3, ali };

    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Passing in custom public keys belonging to drop
// Passing in invalid public keys
// Passing in limit
// Passing in no pub keys and no limit
// Passing in keep empty drop
// Deleting non-empty drop with keys & w/o keys
// Deleting a lot of multi-use keys that are partially used (vast range of keys). I.e 10 use key where 50 keys are deleted, all ranging from 1->10 uses left
// Deleting a drop with a TON of empty asset metadata (check for gas here and make sure it can’t panic)

// Ensure if panic, 
// No keys were deleted
// All state is kept, no data structures touched
// Net state added is zero and no user balance is incremented
// False is returned from function

// Ensure if no panic:
// Keys properly removed from Keypom account
// Keys removed from any data structures
// When drop is fully deleted, any cost that was put in should have been refunded (no more, no less). UNLESS keys have been claimed
// True is returned from function

test('Default - Delete on Empty', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = [{
        assets: [null],
        uses: 1
    }];
    
    // if keep_empty_drop in delete_keys does not work, this will auto-delete
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

    let found_key_info: {owner_id: string, token_id: string, required_gas: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: keypomV3
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)

    //Should not delete drop here
    let deleteResponse = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "delete_keys",
        args: {
            drop_id: dropId,
            public_keys: [keyPairs.publicKeys[0]],
        }
    })
    t.is(deleteResponse=="true", true)

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
});

test('Passing in custom public keys belonging to drop', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = [{
        assets: [null],
        uses: 1
    }];
    
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

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: keypomV3
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)

    let deleteResponse = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "delete_keys",
        args: {
            drop_id: dropId,
            public_keys: [keyPairs.publicKeys[0]],
        }
    })
    t.is(deleteResponse=="true", true)

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
});

test('Passing in invalid public keys', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = [{
        assets: [null],
        uses: 1
    }];
    
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

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: keypomV3
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)
    let preDeleteBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

    let preDeleteKeypomBal = await keypomV3.balance();
    try{
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: "delete_keys",
            args: {
                drop_id: dropId,
                public_keys: ["abcdefghijklmnopqrstuvwxyz123456789"],
            }
        })
        // Should not pass to here, if it does, fail
        t.fail()
    }catch(e){}
    let postDeleteBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let finalBal = await keypomV3.balance();

    t.deepEqual(preDeleteKeypomBal.stateStaked, finalBal.stateStaked)
    t.is(preDeleteBal == postDeleteBal, true)
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), true)
    t.is(await doesDropExist(keypomV3, dropId), true)
});

test('Passing in limit', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 10;
    let keyPairs = await generateKeyPairs(numKeys);

    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = [{
        assets: [null],
        uses: 1
    }];
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: keypomV3
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)

    let deleteResponse = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "delete_keys",
        args: {
            drop_id: dropId,
            limit: 5,
        }
    })
    t.is(deleteResponse=="true", true)

    let allKeys: ExtKeyInfo[] = await keypomV3.view("get_keys_for_drop", {drop_id: dropId})
    t.is(allKeys.length == 5, true)
    t.is(await doesDropExist(keypomV3, dropId), true)
});

test('Passing in keep empty drop', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = [{
        assets: [null],
        uses: 1
    }];
    
    // if keep_empty_drop in delete_keys does not work, this will auto-delete
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

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: keypomV3
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)

    // Shoudl not delete drop here
    let deleteResponse = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "delete_keys",
        args: {
            drop_id: dropId,
            public_keys: [keyPairs.publicKeys[0]],
            keep_empty_drop: true
        }
    })
    t.is(deleteResponse=="true", true)

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)
});

// // 10 use key where 50 keys are deleted, all ranging from 1->10 uses left
test('Deleting a lot of multi-use keys that are partially used', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 50;
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[]= []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    // ******************* Creating Drop *******************
    const asset_data_per_use = [{
        assets: [null],
        uses: 10
    }];
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data,
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string, required_gas: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    console.log(`Required gas: ${found_key_info.required_gas}`)
    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: keypomV3
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)

    for(let i = 0; i < numKeys; i++){
        // Random number between 1-10
        let numClaims: number = Math.floor(Math.random() * (9) + 1)
        console.log(`CLAIMING THE ${i+1}th KEY ${numClaims} TIMES`)
        for(let count = 0; count < numClaims; count++){
            await claimWithRequiredGas({
                keypom: keypomV3,
                keyPair: keyPairs.keys[i],
                root,
                useImplicitAccount: true
            })
        }
        console.log(`~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~`)
    }

    for(let i = 0; i < numKeys; i++){
        let deleteResponse = await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: "delete_keys",
            args: {
                drop_id: dropId,
                public_keys: [keyPairs.publicKeys[i]],
            }
        })
        t.is(deleteResponse=="true", true)
    
        t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[i]), false)
    }

    t.is(await doesDropExist(keypomV3, dropId), false)
});

test(' Deleting a drop with a TON of empty asset metadata - check for gas and ensure no panic', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const asset_data_per_use = [{
        assets: [null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,
            null, null, null, null, null, null, null, null, null,],
        uses: 1
    }];
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
            }]
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string, required_gas: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    console.log(`Required gas: ${found_key_info.required_gas}`)
    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: keypomV3
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)

    let deleteResponse = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "delete_keys",
        args: {
            drop_id: dropId,
            public_keys: [keyPairs.publicKeys[0]],
        }
    })
    t.is(deleteResponse=="true", true)

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
});




// NEED TO COMMENT OUT ALL LOGS INSIDE DELETE_KEYS PRIOR TO RUNNING THIS TEST
// test('Passing in no pub keys and no limit', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
        
//     let initialBal = await keypomV3.balance();
    
//     const dropId = "my-drop-id";
//     const numKeys = 101;
//     let keyPairs = await generateKeyPairs(numKeys);
    
//     let key_data: {public_key: string}[] = []
//     for(let i = 0; i < numKeys - 1; i++){
//         key_data.push({public_key: keyPairs.publicKeys[i]})
//     }
    
//     // ******************* Creating Drop *******************
//     const nearAsset1: ExtNearData = {
//         yoctonear: NEAR.parse("1").toString()
//     }
    
//     const asset_data_per_use = [{
//         assets: [null],
//         uses: 1
//     }];
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: asset_data_per_use,
//             key_data
//         },
//     }) 

//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'add_keys',
//         args: {
//             drop_id: dropId,
//             asset_data: asset_data_per_use,
//             key_data: [{
//                 public_key: keyPairs.publicKeys[100]
//             }]
//         },
//     }) 

//     let addedKeys: ExtKeyInfo[] = await keypomV3.view("get_keys_for_drop", {drop_id: dropId, limit: 150})
//     t.is(addedKeys.length == 101, true)

//     let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
//     let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
//         keypom: keypomV3,
//         expectedTokenId: found_key_info.token_id,
//         keyPair: keyPairs.keys[0],
//         expectedOwner: keypomV3
//     })
//     t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)

//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: "delete_keys",
//         args: {
//             drop_id: dropId,
//         }
//     })

//     let allKeys: ExtKeyInfo[] = await keypomV3.view("get_keys_for_drop", {drop_id: dropId})
//     t.is(allKeys.length == 1, true)
//     t.is(await doesDropExist(keypomV3, dropId), false)
// });

// test('Deleting non-empty drop with keys & w/o keys', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     // ******************* Creating Drop *******************
//     const nearAsset1: ExtNearData = {
//         yoctonear: NEAR.parse("1").toString()
//     }

//     const asset_data_per_use = [{
//         assets: [nearAsset1],
//         uses: 1
//     }];

//     let royalties: Record<string, number>={
//     }
//     // 100000 is 100% 
//     royalties[keypomV3.accountId] = 6500
//     royalties[funder.accountId] = 3000
//     royalties[bob.accountId] = 500

//     let metadata: TokenMetadata = {
//         title: "my token",
//         description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
//         media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
//         media_hash: undefined,
//         copies: undefined,
//         issued_at: undefined,
//         expires_at: undefined,
//         starts_at: undefined,
//         updated_at: undefined,
//         extra: undefined,
//         reference: undefined,
//         reference_hash: undefined
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: asset_data_per_use,
//             key_data: [{
//                 public_key: keyPairs.publicKeys[0],
//             }],
//             drop_config: {
//                 nft_keys_config: {
//                     token_metadata: metadata,
//                     royalties
//                 }
//             }
//         },
//     }) 

//     let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
//     t.is(found_key_info.owner_id == keypomV3.accountId, true)

//     let royalties_and_metadata_same: {royaltySame: boolean, metadataSame: boolean} = await assertNFTKeyData({
//         keypom: keypomV3,
//         tokenId: found_key_info.token_id,
//         expectedRoyalties: royalties,
//         expectedMetadata: metadata
//     })
//     t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), true)

//     let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
//         keypom: keypomV3,
//         expectedTokenId: found_key_info.token_id,
//         keyPair: keyPairs.keys[0],
//         expectedOwner: keypomV3
//     })
//     console.log(storageBools.token_id_by_pk_check)
//     t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)
// });





