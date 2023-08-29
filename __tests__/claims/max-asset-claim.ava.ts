import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertFTBalance, assertKeypomInternalAssets, claimWithRequiredGas, displayBalances, doesDropExist, doesKeyExist, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNearData, InternalNFTData } from "../utils/types";
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
    const ali = await root.createSubAccount('ali');
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    const ftContract = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);

    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"} });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });
    await ftContract.call(ftContract, 'new_default_meta', { owner_id: ftContract, total_supply: totalSupply.toString() });

    await functionCall({signer: ftContract, receiver: ftContract, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract, receiver: ftContract, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract, receiver: ftContract, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, keypomV3, nftContract, ftContract, ali };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// test('Maximum NFTs + Drop Persists Despite Failed Claims + Can Refund Max', async t => {
//     const {funder, keypomV3, nftContract, root, ali} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'add_to_balance',
//         args: {},
//         attachedDeposit: NEAR.parse("10").toString(),
//         shouldLog: false
//     })

//     const nftAsset1 = {
//         nft_contract_id: nftContract.accountId
//     }

//     const dropId = "drop-id";
//     // Max 18 assets
//     const MAX_NUM_NFTS = 18
//     const asset_data = [
//         {
//         // 18 NFT assets - max!
//         assets: Array(MAX_NUM_NFTS).fill(nftAsset1),
//             uses: 1
//         },
//     ]

//     let keyPairs = await generateKeyPairs(1);
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data,
//             key_data: [{
//                 public_key: keyPairs.publicKeys[0],
//             }]
//         }
//     })

//     let token_ids: string[] = []

//     for(let i = 0; i < MAX_NUM_NFTS; i++){
//         let token_id = `token-${i + 1}`

//         await functionCall({
//             signer: funder,
//             receiver: nftContract,
//             methodName: 'nft_mint',
//             args: {
//                 token_id,
//                 metadata: {
//                     title: "my token"
//                 },
//                 receiver_id: funder.accountId
//             },
//             attachedDeposit: NEAR.parse("1").toString(),
//             shouldLog: false
//         })


//         // This token exists!!
//         await functionCall({
//             signer: funder,
//             receiver: nftContract,
//             methodName: 'nft_transfer_call',
//             args: {
//                 receiver_id: keypomV3.accountId,
//                 token_id,
//                 msg: dropId
//             },
//             attachedDeposit: "1"
//         })

//         token_ids.push(token_id)
//     }
    

//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedNftData: [{
//             contract_id: nftContract.accountId,
//             token_ids
//         }],
//     })

//     // Force fail CAAC, trigger asset transfer back to funder, then make sure refunds are all successful
//     let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//         receiverId: ali.accountId,
//         shouldPanic: true
//     })
//     t.is(result.response, "false")

//     // Key should be deleted but drop should persist as assets still exist
//     t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
//     t.is(await doesDropExist(keypomV3, dropId), true)
//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedNftData: [{
//             contract_id: nftContract.accountId,
//             token_ids
//         }],
//     })

//     // Withdraw, then drop should be deleted
//     let response = await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'withdraw_nft_tokens',
//         args: {
//             drop_id: dropId,
//             nft_contract_id: nftContract.accountId,
//             token_ids
//         },
//     })
//     t.is(response, "true");

//     for(let i = 0; i < MAX_NUM_NFTS; i++){
//         // Ensure all refunded NFTs belong to original owner
//         let token: {token_id: string, owner_id: string} = await nftContract.view('nft_token', {token_id: token_ids[i]});
//         t.is(token.owner_id == funder.accountId && token.token_id == token_ids[i], true)
//     }

//     // Delete drop once assets are all back
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'delete_keys',
//         args: {drop_id: dropId},
//         gas: LARGE_GAS,
//         attachedDeposit: "0"
//     })
//     t.is(await doesDropExist(keypomV3, dropId), false)

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });

test('Maximum FTs + Drop Persists Despite Failed Claims + Can Refund Max', async t => {
    const {funder, keypomV3, nftContract, ftContract, root, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_to_balance',
        args: {},
        attachedDeposit: NEAR.parse("10").toString(),
        shouldLog: false
    })

    const FT_AMOUNT_NUM = 5

    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract.accountId,
        registration_cost: NEAR.parse("0.5").toString(),
        ft_amount: FT_AMOUNT_NUM.toString()
    }

    const dropId = "drop-id";
    const MAX_NUM_FT_ASSETS = 12
    const asset_data = [
        {
            // 18 NFT assets - max!
            assets: Array(MAX_NUM_FT_ASSETS).fill(ftAsset1),
            uses: 1
        },
    ]

    let keyPairs = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
            }]
        }
    })

    const keyInfo: {required_gas: string} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    console.log(keyInfo.required_gas)

    let funderInitialFtBal = await ftContract.view('ft_balance_of', {account_id: funder.accountId});
    await sendFTs(funder, (FT_AMOUNT_NUM * MAX_NUM_FT_ASSETS).toString(), keypomV3, ftContract, dropId)

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [
            {
                contract_id: ftContract.accountId,
                balance_avail: (FT_AMOUNT_NUM * MAX_NUM_FT_ASSETS).toString()
            }
        ]
    })

    // Force fail CAAC, trigger asset transfer back to funder, then make sure refunds are all successful
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        receiverId: ali.accountId,
        shouldPanic: true
    })
    t.is(result.response, "false")

    // Key should be deleted but drop should persist as assets still exist
    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [
            {
                contract_id: ftContract.accountId,
                balance_avail: (FT_AMOUNT_NUM * MAX_NUM_FT_ASSETS).toString()
            }
        ]
    })

    await assertFTBalance({
        ftContract: ftContract,
        accountId: keypomV3.accountId,
        amountOwned: (FT_AMOUNT_NUM * MAX_NUM_FT_ASSETS).toString()
    });

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'withdraw_ft_balance',
        args: {
            drop_id: dropId, 
            ft_contract_id: ftContract.accountId, 
            tokens_to_withdraw: (FT_AMOUNT_NUM * MAX_NUM_FT_ASSETS).toString()
        },
        gas: LARGE_GAS,
        attachedDeposit: "0"
    })

    await assertFTBalance({
        ftContract: ftContract,
        accountId: keypomV3.accountId,
        amountOwned: "0"
    });

    let funderFinalFtBal = await ftContract.view('ft_balance_of', {account_id: funder.accountId});
    t.is(funderInitialFtBal, funderFinalFtBal)

    // Delete drop once assets are all back
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'delete_keys',
        args: {drop_id: dropId},
        gas: LARGE_GAS,
        attachedDeposit: "0"
    })
    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});