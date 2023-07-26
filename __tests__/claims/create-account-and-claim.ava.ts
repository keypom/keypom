import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNFTData, InternalNFTData } from "../utils/types";
import { formatNearAmount } from "near-api-js/lib/utils/format";
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
    t.context.accounts = { root, funder, keypomV3, nftContract, ftContract1, ali };
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


// - account creation failed -> should refund ALL assets & key is decremented
// - account creation succeeded but asset claims failed -> should refund assets that failed and do nothing for ones that weren’t

// account creation failed -> should refund ALL assets & key is decremented (NFT version)
// test('Account Creation Fail in CAAC - drop still contains NFTs', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     const nftAsset1 = {
//         nft_contract_id: nftContract.accountId
//     }

//     const asset_data_per_use = {
//         1: {
//             assets: [nftAsset1]
//         },
//         2: {
//             assets: [null]
//         }
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data_per_use,
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
//     let result: {response: string|undefined, actualReceiverId: string | undefined} = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//         receiverId: ali.accountId,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

//     // Key uses should have decremented
//     keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 1)

//     // Second failed claim
//     result = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//         receiverId: ali.accountId,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

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

// account creation failed -> should refund ALL assets & key is decremented (FT and NEAR asset version)
// test('Account Creation Fail in CAAC - drop still contains FTs, refund NEAR from FT and NEAR assets', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nearAssetAmount: number = 1
//     const ftRegistrationCost: number =  1

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     const ftAsset1: ExtFTData = {
//         ft_contract_id: ftContract1.accountId,
//         registration_cost: NEAR.parse("1").toString(),
//         ft_amount: ftRegistrationCost.toString()
//     }

//     const nearAsset1 = {
//         yoctonear: NEAR.parse(nearAssetAmount.toString()).toString()
//     }

//     const asset_data_per_use = {
//         1: {
//             assets: [ftAsset1, nearAsset1]
//         },
//         2: {
//             assets: [null]
//         }
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data_per_use,
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

//     // starting claim NEAR and FT balance
//     let preClaimFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
//     let userPreClaimFTBal= await ftContract1.view("ft_balance_of", {account_id: ali.accountId});
//     let userPreClaimNEARBal: {available: NEAR} = await ali.balance()

//     // First failed claim
//     let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//         receiverId: ali.accountId,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

//     // Key uses should have decremented
//     keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 1)

//     // Ensure no asset transfer
//     let userPostClaimFTBal = await ftContract1.view("ft_balance_of", {account_id: ali.accountId});
//     let userPostClaimNEARBal: {available: NEAR} = await ali.balance()
//     console.log(`Pre NEAR: ${userPreClaimNEARBal.available} and Post NEAR: ${userPostClaimNEARBal.available}`)
//     t.is(userPreClaimFTBal == userPostClaimFTBal, true)
//     t.is(userPreClaimNEARBal.available.toString() == userPostClaimNEARBal.available.toString(), true)

//     // NEAR asset and FT registration cost should have been refunded here
//     let refundedFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
//     let refund = formatNearAmount((BigInt(refundedFunderBal) - BigInt(preClaimFunderBal)).toString(), 5);
//     const EXPECTED_REFUND_NEAR: number = ftRegistrationCost + nearAssetAmount;
//     t.is(refund.toString() >= EXPECTED_REFUND_NEAR.toString(), true)

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
//     result = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//         receiverId: ali.accountId,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

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

// test('Sanity Check', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nearAssetAmount: number = 1
//     const ftRegistrationCost: number =  1

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     const ftAsset1: ExtFTData = {
//         ft_contract_id: ftContract1.accountId,
//         registration_cost: NEAR.parse("1").toString(),
//         ft_amount: ftRegistrationCost.toString()
//     }

//     // bunch of asset IDs
//     const nftAssets: ExtNFTData[] = []
//     const expectedNFTs: InternalNFTData[] = []
//     // 12 was ok with long
//     for(let i = 1; i <= 14; i++){
//         let num: string = (i < 10) ? '0' + i.toString() : i.toString();
//         // let new_contract_id: string = num + "1690235236996169023523699616902352369961690235236996.test.near";
//         let new_contract_id: string = num + "abc.test.near";
//         nftAssets.push({nft_contract_id: new_contract_id})
//         expectedNFTs.push({contract_id: new_contract_id, token_ids: []})
//     }
//     console.log(nftAssets)

//     const asset_data_per_use = {
//         // Max amount of spoof NFTs that will still pass
//         1: {
//             assets: nftAssets
//         },
//         // Max number of FT assets under 300T in estimation
//         2: {
//             assets: [ftAsset1, ftAsset1, ftAsset1, ftAsset1, ftAsset1,
//                      ftAsset1, ftAsset1, ftAsset1, ftAsset1]
//         },
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data_per_use,
//             public_keys: [keyPairs.publicKeys[0]]
//         },
//         attachedDeposit: NEAR.parse("1").toString(),
//     })

//     await sendFTs(funder, "10", keypomV3, ftContract1, dropId)
    

//     // Assert Assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedFtData: [{
//             contract_id: ftContract1.accountId,
//             balance_avail: '10',
//         }],
//         expectedNftData: expectedNFTs,
//     })

//     // 2 uses at the start
//     let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 2)

//     // starting claim NEAR and FT balance
//     let preClaimFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
//     let userPreClaimFTBal= await ftContract1.view("ft_balance_of", {account_id: ali.accountId});
//     let userPreClaimNEARBal: {available: NEAR} = await ali.balance()

//     // First failed claim
//     let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//         receiverId: ali.accountId,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

//     // Key uses should have decremented
//     keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 1)

//     // Second failed claim
//     result = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//         receiverId: ali.accountId,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

//     // Key should be deleted but drop should still exist
//     t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
//     t.is(await doesDropExist(keypomV3, dropId), true)

// });

test('NFT Const Tweaking', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nearAssetAmount: number = 1
    const ftRegistrationCost: number =  1

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // TO DO: TWEAK NFT CONSTANTS SUCH THAT DROPS CREATED = DROPS CLAIMABLE
    // MAKE SURE THAT THIS IS THE ISSUE AS WELL, AND NOT FAILING ELSEWHERE
    // LACK OF LOGS FROM ON_ACCOUNT_CREATED IS CONCERNING

    // bunch of asset IDs
    const nftAssets: ExtNFTData[] = []
    const expectedNFTs: InternalNFTData[] = []
    // 12 was ok with long
    for(let i = 1; i <= 15; i++){
        let num: string = (i < 10) ? '0' + i.toString() : i.toString();
        let new_contract_id: string = num + "1690235236996169023523699616902352369961690235236996.test.near";
        // let new_contract_id: string = num + "abc.test.near";
        nftAssets.push({nft_contract_id: new_contract_id})
        expectedNFTs.push({contract_id: new_contract_id, token_ids: []})
    }

    const asset_data_per_use = {
        // Max amount of spoof NFTs that will still pass226/18
        1: {
            assets: nftAssets
        },
        2: {
            assets: [null]
        }
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data_per_use,
            public_keys: [keyPairs.publicKeys[0]]
        },
        attachedDeposit: NEAR.parse("1").toString(),
    })
    
    // Assert Assets
    // await assertKeypomInternalAssets({
    //     keypom: keypomV3,
    //     dropId,
    //     expectedNftData: expectedNFTs,
    // })

    // First legit claim to get to 2nd use
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
    })

    result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        receiverId: ali.accountId,
        shouldPanic: true
    })
    t.is(result.response, "false")

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)

});


// account creation succeeded but asset claims failed -> should refund assets that failed and do nothing for ones that weren’t
// test('Asset Claim Failure in CAAC', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const nearAssetAmount: number = 1
//     const ftRegistrationCost: number =  1

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     const ftAsset1: ExtFTData = {
//         ft_contract_id: ftContract1.accountId,
//         registration_cost: NEAR.parse("1").toString(),
//         ft_amount: ftRegistrationCost.toString()
//     }

//     const ftAsset2: ExtFTData = {
//         ft_contract_id: "foo-contract",
//         registration_cost: NEAR.parse("1").toString(),
//         ft_amount: '0'
//     }

//     const nearAsset1 = {
//         yoctonear: NEAR.parse(nearAssetAmount.toString()).toString()
//     }

//     const asset_data_per_use = {
//         1: {
//             assets: [ftAsset1, nearAsset1, ftAsset2]
//         },
//         2: {
//             assets: [null]
//         }
//     }
    
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data_per_use,
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
//         },
//         {
//             contract_id: "foo-contract",
//             balance_avail: '0'
//         }
//     ]
//     })

//     // 2 uses at the start
//     let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 2)

//     // starting claim balance
//     let preClaimFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

//     // First claim
//     let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

//     // Key uses should have decremented
//     keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 1)

//     // Ensure proper asset transfer
//     let userPostClaimFTBal: number = await ftContract1.view("ft_balance_of", {account_id: result.actualReceiverId});
//     // CAAC, starting FT bal was 0
//     t.is(userPostClaimFTBal == 1, true)
    
//     //ensure refund from failed ft claim is sent back, but NOT near asset
//     let refundedFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
//     let refund = formatNearAmount((BigInt(refundedFunderBal) - BigInt(preClaimFunderBal)).toString(), 5);
//     console.log(`Refunded Amount: ${refund}`)
//     const EXPECTED_REFUND_NEAR: number = ftRegistrationCost;
//     // Ensure NEAR asset was not refunded
//     t.is(refund.toString() >= EXPECTED_REFUND_NEAR.toString() && refund.toString() < (EXPECTED_REFUND_NEAR + nearAssetAmount).toString(), true)

//     // Second claim, check refund of next registration cost

// });

// test('Account Creation Fail in CAAC - Refund Registration and NEAR Asset Costs', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 1;
//     // Always create a new keypair for CAAC
//     let keyPairs = await generateKeyPairs(numKeys);

//     const ftAsset1: ExtFTData = {
//         ft_contract_id: ftContract1.accountId,
//         registration_cost: NEAR.parse("1").toString(),
//         ft_amount: "1"
//     }

//     const nftAsset1 = {
//         nft_contract_id: nftContract.accountId
//     }

//     const nearAsset1 = {
//         yoctonear: NEAR.parse("1").toString()
//     }

//     // FT claim to work: no more assets and thus drop deleted
//     // Dont let near asset claim to fail - expect a refund here as well
//     const assets_per_use = {
//         1: [ftAsset1],
//         2: [nearAsset1]
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
//         attachedDeposit: NEAR.parse("3").toString(),
//     })

//     // let tokenIds: string[]= [];
//     // let numTokens = 1;
//     // for (let i = 1; i < numTokens+1; i++) {
//     //     let tokenId = `token-1-${i}`

//     //     await functionCall({
//     //         signer: funder,
//     //         receiver: nftContract,
//     //         methodName: 'nft_mint',
//     //         args: {
//     //             token_id: tokenId,
//     //             metadata: {
//     //                 title: "my token"
//     //             },
//     //             receiver_id: funder.accountId
//     //         },
//     //         attachedDeposit: NEAR.parse("0.01").toString(),
//     //         shouldLog: true
//     //     })

//     //     await functionCall({
//     //         signer: funder,
//     //         receiver: nftContract,
//     //         methodName: 'nft_transfer_call',
//     //         args: {
//     //             receiver_id: keypomV3.accountId,
//     //             token_id: tokenId,
//     //             msg: dropId
//     //         },
//     //         attachedDeposit: "1"
//     //     })

//     //     tokenIds.push(tokenId)
//     // }

//     await sendFTs(funder, "1", keypomV3, ftContract1, dropId)
    

//     // Assert Assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedFtData: [{
//             contract_id: ftContract1.accountId,
//             balance_avail: '1',
//         }],
//         // expectedNftData:[{
//         //     contract_id: nftContract.accountId,
//         //     token_ids: tokenIds
//         // }]
//     })

//     // 2 uses at the start
//     let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 2)

//     let startingFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

//     // First successful claim
//     let result: {response: string|undefined, newAccountId: string} = await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0],
//         createAccount: true,
//     })

//     // Key uses should have decremented
//     keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
//     t.is(keyInfo.uses_remaining, 1)

//     // Assert Assets
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedFtData: [{
//             contract_id: ftContract1.accountId,
//             balance_avail: '0',
//         }],
//         // expectedNftData:[{
//         //     contract_id: nftContract.accountId,
//         //     token_ids: []
//         // }]
//     })

//     // Second failed claim
//     result = await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0],
//         createAccount: true,
//         newAccountId: root.accountId,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

//     // Key should be deleted but drop should still exist
//     t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
//     t.is(await doesDropExist(keypomV3, dropId), false)

//     let endingFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
//     // refund amount, rounded to 5 decimal places
//     let refund = BigInt(endingFunderBal) - BigInt(startingFunderBal);
//     let rounded = formatNearAmount(refund.toString(), 5)
//     const DROP_DEPOSIT: number = 0.01621
//     // 1 NEAR from failed NEAR asset claim
//     const EXPECTED_REFUND_NEAR: number = 1 + DROP_DEPOSIT;
//     t.is(rounded.toString() == EXPECTED_REFUND_NEAR.toString(), true)
// });

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
//     let result: {response: string|undefined, newAccountId: string} = await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0]
//     })

//     // After a succesful claim, Keypom keys should be back to just the one FAK
//     let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys.keys.length, 1);

// });
