import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNFTData, InternalNFTData, InternalFTData } from "../utils/types";
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
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);

    // ************** NFT SETUP **************
    // let nftAccounts: NearAccount[] = []

    // const nftContract1 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract1)
    // const nftContract2 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract2)
    // const nftContract3 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract3)
    // const nftContract4 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract4)
    // const nftContract5 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract5)
    // const nftContract6 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract6)
    // const nftContract7 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract7)
    // const nftContract8 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract8)
    // const nftContract9 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract9)
    // const nftContract10 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract10)
    // const nftContract11 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract11)
    // const nftContract12 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract12)
    // const nftContract13 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract13)
    // const nftContract14 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract14)
    // const nftContract15 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract15)
    // const nftContract16 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract16)
    // const nftContract17 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // nftAccounts.push(nftContract17)
    // // const nftContract18 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // // nftAccounts.push(nftContract18)
    //
    // for(let i = 0; i < nftAccounts.length; i++){
    //     await nftAccounts[i].call(nftAccounts[i], 'new_default_meta', { owner_id: nftAccounts[i]});
    //     await functionCall({
    //         signer: funder,
    //         receiver: nftAccounts[i],
    //         methodName: 'nft_mint',
    //         args: {
    //             token_id: "token1",
    //             metadata: {
    //                 title: "my token"
    //             },
    //             receiver_id: funder.accountId
    //         },
    //         attachedDeposit: NEAR.parse("1").toString(),
    //         shouldLog: false
    //     })
    // }
    
     // ************** FT Setup **************
     let ftAccounts: NearAccount[] = []

     const ftContract1 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);

     const ftContract2 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
     const ftContract3 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
     const ftContract4 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
     const ftContract5 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
     const ftContract6 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
     const ftContract7 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
     const ftContract8 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
     const ftContract9 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);

    //  const ftContract10 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    //  const ftContract11 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    //  const ftContract12 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    //  const ftContract13 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    //  const ftContract14 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    //  const ftContract15 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    //  const ftContract16 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    //  const ftContract17 = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);

     ftAccounts.push(ftContract1)

     ftAccounts.push(ftContract2)
     ftAccounts.push(ftContract3)
     ftAccounts.push(ftContract4)
     ftAccounts.push(ftContract5)
     ftAccounts.push(ftContract6)
     ftAccounts.push(ftContract7)
     ftAccounts.push(ftContract8)
     ftAccounts.push(ftContract9)

    //  ftAccounts.push(ftContract10)
    //  ftAccounts.push(ftContract11)
    //  ftAccounts.push(ftContract12)
    //  ftAccounts.push(ftContract13)
    //  ftAccounts.push(ftContract14)
    //  ftAccounts.push(ftContract15)
    //  ftAccounts.push(ftContract16)
    //  ftAccounts.push(ftContract17)

    for(let i = 0; i < ftAccounts.length; i++){
        console.log(`Initializing ${ftAccounts[i].accountId}`)
        await ftAccounts[i].call(ftAccounts[i], 'new_default_meta', { owner_id: ftAccounts[i], total_supply: totalSupply.toString() });
        await functionCall({signer: ftAccounts[i], receiver: ftAccounts[i], methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("10").toString(), shouldLog: false})
        await functionCall({signer: ftAccounts[i], receiver: ftAccounts[i], methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("10").toString(), shouldLog: false})
        await functionCall({signer: ftAccounts[i], receiver: ftAccounts[i], methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    }


    
    
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
    // t.context.accounts = { root, funder, keypomV3, ali };
    // t.context.accounts = { root, funder, keypomV3, 
    //     nftContract1, 
    //     nftContract2, 
    //     nftContract3, 
    //     nftContract4, 
    //     nftContract5, 
    //     nftContract6, 
    //     nftContract7, 
    //     nftContract8, 
    //     nftContract9, 
    //     nftContract10, 
    //     nftContract11, 
    //     nftContract12, 
    //     nftContract13, 
    //     nftContract14, 
    //     nftContract15, 
    //     nftContract16,
    //     nftContract17,
    //     ali };
    t.context.accounts = { root, funder, keypomV3, 
        ftContract1, 

        ftContract2, 
        ftContract3, 
        ftContract4, 
        ftContract5, 
        ftContract6, 
        ftContract7, 
        ftContract8, 
        ftContract9, 

        // ftContract10, 
        // ftContract11, 
        // ftContract12, 
        // ftContract13, 
        // ftContract14, 
        // ftContract15, 
        // ftContract16,
        // ftContract17,
        ali };

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
//     t.is(await doesDropExist(keypomV3, dropId), false)

// });

// test('NFT Const Tweaking', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, 
//         nftContract2, 
//         nftContract3, 
//         nftContract4, 
//         nftContract5, 
//         nftContract6, 
//         nftContract7, 
//         nftContract8, 
//         nftContract9, 
//         nftContract10, 
//         nftContract11, 
//         nftContract12, 
//         nftContract13, 
//         nftContract14, 
//         nftContract15, 
//         nftContract16,
//         nftContract17,
//          ali} = t.context.accounts;
    
//     let nftAccounts: NearAccount[] = [ nftContract1, 
//         nftContract2, 
//         nftContract3, 
//         nftContract4, 
//         nftContract5, 
//         nftContract6, 
//         nftContract7, 
//         nftContract8, 
//         nftContract9, 
//         nftContract10, 
//         nftContract11, 
//         nftContract12, 
//         nftContract13, 
//         nftContract14, 
//         nftContract15, 
//         nftContract16,
//         nftContract17,
//         ]
    
//     let initialBal = await keypomV3.balance();

//     const nearAssetAmount: number = 1
//     const ftRegistrationCost: number =  1

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     // ******************* EMPTIES *******************
//     // const nftAssets: ExtNFTData[] = []
//     // const expectedNFTs: InternalNFTData[] = []
//     // for(let i = 1; i <= 17; i++){
//     //     let num: string = (i < 10) ? '0' + i.toString() : i.toString();
//     //     let new_contract_id: string = num + "1690235236996169023523699616902352369961690235236996.test.near";
//     //     // let new_contract_id: string = num + "abc.test.near";
//     //     nftAssets.push({nft_contract_id: new_contract_id})
//     //     expectedNFTs.push({contract_id: new_contract_id, token_ids: []})
//     // }

//     // const asset_data_per_use = {
//     //     // Max amount of spoof NFTs that will still pass226/18
//     //     1: {
//     //         assets: nftAssets
//     //     },
//     // }

//     // ******************* ACTUAL *******************
//     // const nftAsset1: ExtNFTData = {
//     //     nft_contract_id: nftContract1.accountId
//     // }

//     // const asset_data_per_use = {
//     //     // Max amount of spoof NFTs that will still pass226/18
//     //     1: {
//     //         assets: Array(17).fill(nftAsset1)
//     //     },
//     // }

//     // ******************* BIG BOY *******************
//     let nftAssets: ExtNFTData[] = [];
//     let expectedNftAssets: InternalNFTData[] = [];
//     for(let i = 0 ; i < nftAccounts.length; i++){
//         nftAssets.push({nft_contract_id: nftAccounts[i].accountId})
//         expectedNftAssets.push({contract_id: nftAccounts[i].accountId, token_ids: ["token1"]})
//     }

//     const asset_data_per_use = {
//         // Max amount of spoof NFTs that will still pass226/18
//         1: {
//             assets: nftAssets
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

    
     
//     // ******************* ACTUAL *******************
//     // let tokenIds: string[] = [];

//     // for(let i = 1; i<=17; i++){
//     //     // let tokenId = `token-1-${i}`
//     //     let tokenId = `token1`

//     //     await functionCall({
//     //         signer: funder,
//     //         receiver: nftContract1,
//     //         methodName: 'nft_mint',
//     //         args: {
//     //             token_id: tokenId,
//     //             metadata: {
//     //                 title: "my token",
//     //                 description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
//     //                 media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
//     //             },
//     //             receiver_id: funder.accountId
//     //         },
//     //         attachedDeposit: NEAR.parse("0.01").toString(),
//     //         shouldLog: true
//     //     })

//     //     await functionCall({
//     //         signer: funder,
//     //         receiver: nftContract1,
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

//     // ******************* BIG BOY *******************
//     let tokenIds: string[] = [];

//     for(let i = 0; i < nftAccounts.length; i++){
//         // let tokenId = `token-1-${i}`
//         let tokenId = `token1`

//         await functionCall({
//             signer: funder,
//             receiver: nftAccounts[i],
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
    
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedNftData: expectedNftAssets,
//     })

//     // ******************* EMPTIES *******************
//     // await assertKeypomInternalAssets({
//     //     keypom: keypomV3,
//     //     dropId,
//     //     expectedNftData: expectedNFTs,
//     // })

//     // First legit claim to get to 2nd use
//     let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//     })
//     t.is(result.response, "true")

//     // Key should be deleted but drop should still exist
//     t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
//     t.is(await doesDropExist(keypomV3, dropId), true)

// });

test('FT Const Tweaking', async t => {
    const {funder, keypomV3, root,  
        ftContract1, 
        ftContract2, 
        ftContract3, 
        ftContract4, 
        ftContract5, 
        ftContract6, 
        ftContract7, 
        ftContract8, 
        ftContract9, 
        ftContract10, 
        ftContract11, 
        ftContract12, 
        ftContract13, 
        ftContract14, 
        ftContract15, 
        ftContract16,
        ftContract17,
         ali} = t.context.accounts;
    
    let ftAccounts: NearAccount[] = [ 
        ftContract1, 
        ftContract2, 
        ftContract3, 
        ftContract4, 
        ftContract5, 
        ftContract6, 
        ftContract7, 
        ftContract8, 
        ftContract9, 
        ]
    
    let initialBal = await keypomV3.balance();

    const nearAssetAmount: number = 1
    const ftRegistrationCost: number =  1

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);
    let under_max = 2;

    // ******************* EMPTIES *******************
    // let ftAssets: ExtFTData[] = []
    // let expectedFTs: {contract_id: string, balance_avail: string}[] = []
    // for(let i = 1; i <= 9; i++){
    //     let num: string = (i < 10) ? '0' + i.toString() : i.toString();
    //     let new_contract_id: string = num + "1690235236996169023523699616902352369961690235236996.test.near";
    //     // let new_contract_id: string = num + "abc.test.near";
    //     ftAssets.push({ft_contract_id: new_contract_id, registration_cost: "0", ft_amount: "0"})
    //     expectedFTs.push({contract_id: new_contract_id, balance_avail: "0"})
    // }

    // const asset_data_per_use = {
    //     // Max amount of spoof NFTs that will still pass226/18
    //     1: {
    //         assets: ftAssets
    //     },
    // }

    // ******************* ACTUAL *******************
    // const nftAsset1: ExtNFTData = {
    //     nft_contract_id: nftContract1.accountId
    // }

    // const asset_data_per_use = {
    //     // Max amount of spoof NFTs that will still pass226/18
    //     1: {
    //         assets: Array(17).fill(nftAsset1)
    //     },
    // }

    // ******************* BIG BOY *******************
    let ftAssets: ExtFTData[] = [];
    let expectedFtAssets: {contract_id: string, balance_avail: string}[] = [];
    for(let i = 0 ; i < ftAccounts.length - under_max; i++){
        ftAssets.push({ft_contract_id: ftAccounts[i].accountId, registration_cost: NEAR.parse("1").toString(), ft_amount: "1"});
        expectedFtAssets.push({contract_id: ftAccounts[i].accountId, balance_avail: "1"})
    }

    const asset_data_per_use = [
        // Max amount of spoof NFTs that will still pass226/18
        {
            assets: ftAssets
        },
    ]
    
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

     
    // ******************* ACTUAL *******************
    // let tokenIds: string[] = [];

    // for(let i = 1; i<=17; i++){
    //     // let tokenId = `token-1-${i}`
    //     let tokenId = `token1`

    //     await functionCall({
    //         signer: funder,
    //         receiver: nftContract1,
    //         methodName: 'nft_mint',
    //         args: {
    //             token_id: tokenId,
    //             metadata: {
    //                 title: "my token",
    //                 description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
    //                 media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
    //             },
    //             receiver_id: funder.accountId
    //         },
    //         attachedDeposit: NEAR.parse("0.01").toString(),
    //         shouldLog: true
    //     })
 
    //     await functionCall({
    //         signer: funder,
    //         receiver: nftContract1,
    //         methodName: 'nft_transfer_call',
    //         args: {
    //             receiver_id: keypomV3.accountId,
    //             token_id: tokenId,
    //             msg: dropId
    //         },
    //         attachedDeposit: "1"
    //     })

    //     tokenIds.push(tokenId)
    // }

    // ******************* BIG BOY *******************
    for(let i = 0; i < ftAccounts.length - under_max; i++){
        console.log(`Sending FTs from ${ftAccounts[i].accountId}`)
        await sendFTs(funder, "1", keypomV3, ftAccounts[i], dropId)
    }
    
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: expectedFtAssets
    })

    // ******************* EMPTIES *******************
    // await assertKeypomInternalAssets({
    //     keypom: keypomV3,
    //     dropId,
    //     expectedFtData: expectedFtAssets,
    //     expectedNftData: []
    // })

    // First legit claim to get to 2nd use
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)

});


