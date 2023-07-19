import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, InternalNFTData } from "../utils/types";
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

    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // const nftContract2 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);

    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });
    // await nftContract2.call(nftContract2, 'new_default_meta', { owner_id: nftContract2 });


    const ftContract1 = await root.createSubAccount('ft_contract_1');
    // const ftContract2 = await root.createSubAccount('ft_contract_2');


    await ftContract1.deploy(`./__tests__/ext-wasm/ft.wasm`);
    // await ftContract2.deploy(`./__tests__/ext-wasm/ft.wasm`);

    
    await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });
    // await ftContract2.call(ftContract2, 'new_default_meta', { owner_id: ftContract2, total_supply: totalSupply.toString() });


    // Deposit storage
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})


    // Send FTs
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})


    // await functionCall({
    //     signer: funder,
    //     receiver: nftContract,
    //     methodName: 'nft_mint',
    //     args: {
    //         token_id: "token1",
    //         metadata: {
    //             title: "my token"
    //         },
    //         receiver_id: funder.accountId
    //     },
    //     attachedDeposit: NEAR.parse("1").toString(),
    //     shouldLog: false
    // })

    // await functionCall({
    //     signer: funder,
    //     receiver: nftContract2,
    //     methodName: 'nft_mint',
    //     args: {
    //         token_id: "token1",
    //         metadata: {
    //             title: "my token"
    //         },
    //         receiver_id: funder.accountId
    //     },
    //     attachedDeposit: NEAR.parse("1").toString(),
    //     shouldLog: false
    // })

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
    t.context.accounts = { root, funder, keypomV3, nftContract, ftContract1 };
    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// EXPECTED CAAC FAILIURE BEHAVIOUR
// Failed CAAC:
// - no assets transferred
// - Check if key is properly deleted in the case that it’s 0 remaining uses and check if it isn’t deleted in other case
// - Make sure drop is deleted ONLY if it’s empty of keys and has no funded assets in it (i.e FTs & NFTs)
// - If drop is deleted, only NEAR assets and FT registration cost should be refunded back to user balance


// account creation failed -> should refund ALL assets & key is decremented
// test('Account Creation Fail in CAAC and drop still contains NFTs', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     const nftAsset1 = {
//         nft_contract_id: nftContract.accountId
//     }

//     const assets_per_use = {
//         1: [nftAsset1],
//         2: [null]
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: [keyPairs.publicKeys[0]]
//         },
//         attachedDeposit: NEAR.parse("1").toString(),
//     })

//     // 12TGas for NFT asset, 20 tokens + other gas = 293TGas
//     let tokenIds: string[]= [];
//     let numTokens = 1;
//     for (let i = 1; i < numTokens+1; i++) {
//         let tokenId = `token-1-${i}`

//         await functionCall({
//             signer: funder,
//             receiver: nftContract,
//             methodName: 'nft_mint',
//             args: {
//                 token_id: tokenId,
//                 metadata: {
//                     title: "my token"
//                 },
//                 receiver_id: funder.accountId
//             },
//             attachedDeposit: NEAR.parse("0.01").toString(),
//             shouldLog: true
//         })

//         await functionCall({
//             signer: funder,
//             receiver: nftContract,
//             methodName: 'nft_transfer_call',
//             args: {
//                 receiver_id: keypomV3.accountId,
//                 token_id: tokenId,
//                 msg: dropId
//             },
//             attachedDeposit: "1"
//         })

//         tokenIds.push(tokenId)
//     }

//     // Assert Assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedNftData: [{
//             contract_id: nftContract.accountId,
//             token_ids: tokenIds
//         },],
//     })

//     // 2 uses at the start
//     let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 2)

//     // First failed claim
//     let response = await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0],
//         createAccount: true,
//         newPublicKey: keyPairs.publicKeys[keyPairs.publicKeys.length - 1],
//         newAccountId: root.accountId,
//         shouldPanic: true
//     })
//     t.is(response, "false")

//     // Key uses should have decremented
//     keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 1)

//     // Second failed claim
//     response = await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0],
//         createAccount: true,
//         newPublicKey: keyPairs.publicKeys[keyPairs.publicKeys.length - 1],
//         newAccountId: root.accountId,
//         shouldPanic: true
//     })
//     t.is(response, "false")

//     // Token should still be owned by Keypom
//     let token: {token_id: string, owner_id: string} = await nftContract.view('nft_token', {token_id: tokenIds[0]});
//     console.log(`${token.token_id} is owned by ${token.owner_id}`)
//     t.is(token.owner_id, keypomV3.accountId)

//     // Key should be deleted but drop should still exist
//     t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
//     t.is(await doesDropExist(keypomV3, dropId), true)

//     // Drop should still have assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedNftData: [{
//             contract_id: nftContract.accountId,
//             token_ids: tokenIds
//         },],
//     })

// });
// test('Account Creation Fail in CAAC and drop still contains FTs', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     const ftAsset1: ExtFTData = {
//         ft_contract_id: ftContract1.accountId,
//         registration_cost: NEAR.parse("0.05").toString(),
//         ft_amount: "1"
//     }

//     const assets_per_use = {
//         1: [ftAsset1],
//         2: [null]
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: [keyPairs.publicKeys[0]]
//         },
//         attachedDeposit: NEAR.parse("1").toString(),
//     })

//     await sendFTs(funder, "1", keypomV3, ftContract1, dropId)
    

//     // Assert Assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedFtData: [{
//             contract_id: ftContract1.accountId,
//             balance_avail: '1',
//         }]
//     })

//     // 2 uses at the start
//     let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 2)

//     // First failed claim
//     let response = await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0],
//         createAccount: true,
//         newPublicKey: keyPairs.publicKeys[keyPairs.publicKeys.length - 1],
//         newAccountId: root.accountId,
//         shouldPanic: true
//     })
//     t.is(response, "false")

//     // Key uses should have decremented
//     keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 1)

//     // Assert Assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedFtData: [{
//             contract_id: ftContract1.accountId,
//             balance_avail: '1',
//         }]
//     })

//     // Second failed claim
//     response = await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0],
//         createAccount: true,
//         newPublicKey: keyPairs.publicKeys[keyPairs.publicKeys.length - 1],
//         newAccountId: root.accountId,
//         shouldPanic: true
//     })
//     t.is(response, "false")

//     // Key should be deleted but drop should still exist
//     t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
//     t.is(await doesDropExist(keypomV3, dropId), true)

//     // Drop should still have assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedFtData: [{
//             contract_id: ftContract1.accountId,
//             balance_avail: '1',
//         }]
//     })

// });

test('Account Creation Fail in CAAC - Refund Registration and NEAR Asset Costs', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse("1").toString(),
        ft_amount: "1"
    }

    const nftAsset1 = {
        nft_contract_id: nftContract.accountId
    }

    const nearAsset1 = {
        yoctonear: NEAR.parse("1").toString()
    }

    // FT claim to work: no more assets and thus drop deleted
    // Dont let near asset claim to fail - expect a refund here as well
    const assets_per_use = {
        1: [nftAsset1],
        2: [nearAsset1]
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_per_use,
            public_keys: [keyPairs.publicKeys[0]]
        },
        attachedDeposit: NEAR.parse("3").toString(),
    })

    let tokenIds: string[]= [];
    let numTokens = 1;
    for (let i = 1; i < numTokens+1; i++) {
        let tokenId = `token-1-${i}`

        await functionCall({
            signer: funder,
            receiver: nftContract,
            methodName: 'nft_mint',
            args: {
                token_id: tokenId,
                metadata: {
                    title: "my token"
                },
                receiver_id: funder.accountId
            },
            attachedDeposit: NEAR.parse("0.01").toString(),
            shouldLog: true
        })

        await functionCall({
            signer: funder,
            receiver: nftContract,
            methodName: 'nft_transfer_call',
            args: {
                receiver_id: keypomV3.accountId,
                token_id: tokenId,
                msg: dropId
            },
            attachedDeposit: "1"
        })

        tokenIds.push(tokenId)
    }

    // await sendFTs(funder, "1", keypomV3, ftContract1, dropId)
    

    // Assert Assets
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        // expectedFtData: [{
        //     contract_id: ftContract1.accountId,
        //     balance_avail: '1',
        // }],
        expectedNftData:[{
            contract_id: nftContract.accountId,
            token_ids: tokenIds
        }]
    })

    // 2 uses at the start
    let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 2)

    let startingFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

    // First successful claim
    let response = await claimWithRequiredGas({
        keypomV3,
        root,
        key: keyPairs.keys[0],
        publicKey: keyPairs.publicKeys[0],
        createAccount: true,
        newPublicKey: keyPairs.publicKeys[keyPairs.publicKeys.length - 1],
    })

    // Key uses should have decremented
    keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 1)

    // Assert Assets
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        // expectedFtData: [{
        //     contract_id: ftContract1.accountId,
        //     balance_avail: '1',
        // }],
        expectedNftData:[{
            contract_id: nftContract.accountId,
            token_ids: []
        }]
    })

    // Second failed claim
    response = await claimWithRequiredGas({
        keypomV3,
        root,
        key: keyPairs.keys[0],
        publicKey: keyPairs.publicKeys[0],
        createAccount: true,
        newPublicKey: keyPairs.publicKeys[keyPairs.publicKeys.length - 1],
        newAccountId: root.accountId,
        shouldPanic: true
    })
    t.is(response, "false")

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)

    let endingFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
    let refund: number = endingFunderBal - startingFunderBal;
    t.is(refund.toString() == NEAR.parse("1").toString(), true)
});

// account creation succeeded but asset claims failed -> should refund assets that failed and do nothing for ones that weren’t
// test('Ensuring drop creation gas limit is accurate', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 1;
//     let keyPairs = await generateKeyPairs(numKeys);

//     const nftAsset1 = {
//         nft_contract_id: nftContract.accountId
//     }

//     const assets_per_use = {
//         1: [nftAsset1, nftAsset1, nftAsset1, nftAsset1, nftAsset1, 
//             nftAsset1, nftAsset1, nftAsset1, nftAsset1, nftAsset1, 
//             nftAsset1, nftAsset1, nftAsset1, nftAsset1, nftAsset1,
//             nftAsset1, nftAsset1, nftAsset1, nftAsset1, nftAsset1]
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: keyPairs.publicKeys
//         },
//         attachedDeposit: NEAR.parse("10").toString(),
//     })

//     // 12TGas for NFT asset, 20 tokens + other gas = 293TGas
//     let tokenIds: string[]= [];
//     let numTokens = 20;
//     for (let i = 1; i < numTokens+1; i++) {
//         let tokenId = `token-1-${i}`

//         await functionCall({
//             signer: funder,
//             receiver: nftContract,
//             methodName: 'nft_mint',
//             args: {
//                 token_id: tokenId,
//                 metadata: {
//                     title: "my token"
//                 },
//                 receiver_id: funder.accountId
//             },
//             attachedDeposit: NEAR.parse("0.01").toString(),
//             shouldLog: true
//         })

//         await functionCall({
//             signer: funder,
//             receiver: nftContract,
//             methodName: 'nft_transfer_call',
//             args: {
//                 receiver_id: keypomV3.accountId,
//                 token_id: tokenId,
//                 msg: dropId
//             },
//             attachedDeposit: "1"
//         })

//         tokenIds.push(tokenId)
//     }

//     // Assert Assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedNftData: [{
//             contract_id: nftContract.accountId,
//             token_ids: tokenIds
//         },],
//     })

//     // Claim
//     await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0]
//     })

//     // After a succesful claim, Keypom keys should be back to just the one FAK
//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

// });
