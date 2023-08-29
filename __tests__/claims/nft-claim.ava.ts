import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, claimWithRequiredGas, displayBalances, doesDropExist, doesKeyExist, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtNearData, InternalNFTData } from "../utils/types";
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
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    const nftContract2 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"} });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });
    await nftContract2.call(nftContract2, 'new_default_meta', { owner_id: nftContract2 });

    await functionCall({
        signer: funder,
        receiver: nftContract,
        methodName: 'nft_mint',
        args: {
            token_id: "token1",
            metadata: {
                title: "my token"
            },
            receiver_id: funder.accountId
        },
        attachedDeposit: NEAR.parse("1").toString(),
        shouldLog: false
    })

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, keypomV3, nftContract, nftContract2 };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// test('Successful Single and Multi Claim', async t => {
//     const {funder, keypomV3, nftContract, nftContract2, root} = t.context.accounts;
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
//     const nftAsset2 = {
//         nft_contract_id: nftContract2.accountId
//     }

//     const dropId = "drop-id";
//     const asset_data = [
//         {
//         assets: [nftAsset1, nftAsset2],
//             uses: 1
//         },
//         {
//             assets: [nftAsset1],
//                 uses: 1
//             }
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

//     // This token exists!!
//     await functionCall({
//         signer: funder,
//         receiver: nftContract,
//         methodName: 'nft_transfer_call',
//         args: {
//             receiver_id: keypomV3.accountId,
//             token_id: "token1",
//             msg: dropId
//         },
//         attachedDeposit: "1"
//     })

//     token_ids.push("token1")

//     // Second NFT on nftContract
//     await functionCall({
//         signer: funder,
//         receiver: nftContract,
//         methodName: 'nft_mint',
//         args: {
//             token_id: "token2",
//             metadata: {
//                 title: "my token"
//             },
//             receiver_id: funder.accountId
//         },
//         attachedDeposit: NEAR.parse("1").toString(),
//         shouldLog: false
//     })

//     await functionCall({
//         signer: funder,
//         receiver: nftContract,
//         methodName: 'nft_transfer_call',
//         args: {
//             receiver_id: keypomV3.accountId,
//             token_id: "token2",
//             msg: dropId
//         },
//         attachedDeposit: "1"
//     })

//     token_ids.push("token2")

//     // First NFT on nftContract2
//     await functionCall({
//         signer: funder,
//         receiver: nftContract2,
//         methodName: 'nft_mint',
//         args: {
//             token_id: "token-2-1",
//             metadata: {
//                 title: "my token"
//             },
//             receiver_id: funder.accountId
//         },
//         attachedDeposit: NEAR.parse("1").toString(),
//         shouldLog: false
//     })

//     await functionCall({
//         signer: funder,
//         receiver: nftContract2,
//         methodName: 'nft_transfer_call',
//         args: {
//             receiver_id: keypomV3.accountId,
//             token_id: "token-2-1",
//             msg: dropId
//         },
//         attachedDeposit: "1"
//     })

//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedNftData: [{
//             contract_id: nftContract.accountId,
//             token_ids
//         },
//         {
//             contract_id: nftContract2.accountId,
//             token_ids: ["token-2-1"]
//         }
//         ],
//     })

//     // First Claim - should transfer 1 NFT from contract 1, another from contract 2
//     let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//     })
//     t.is(result.response, "true")
//     let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId
//     t.is(await (keypomV3.getAccount(claimingAccount)).exists(), true);

//     t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), true)
//     t.is(await doesDropExist(keypomV3, dropId), true)

//     // First contract NFT
//     let token: {token_id: string, owner_id: string} = await nftContract.view('nft_token', {token_id: token_ids[1]});
//     t.is(token.owner_id == claimingAccount, true)
//     // Second contract NFT
//     let token2: {token_id: string, owner_id: string} = await nftContract2.view('nft_token', {token_id: "token-2-1"});
//     t.is(token2.owner_id == claimingAccount, true)

//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId,
//         expectedNftData: [{
//             contract_id: nftContract.accountId,
//             token_ids: [token_ids[0]]
//         },
//         {
//             contract_id: nftContract2.accountId,
//             token_ids: []
//         }
//         ],
//     })

//     // Second claim, just a single NFT from first contract
//     result = await claimWithRequiredGas({
//         keypom: keypomV3,
//         root,
//         keyPair: keyPairs.keys[0],
//         createAccount: true,
//     })
//     t.is(result.response, "true")
//     claimingAccount = result.actualReceiverId == undefined ? "" : result.actualReceiverId
//     t.is(await (keypomV3.getAccount(claimingAccount)).exists(), true);

//     t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), false)
//     t.is(await doesDropExist(keypomV3, dropId), false)

//     // First contract NFT
//     token = await nftContract.view('nft_token', {token_id: token_ids[0]});
//     t.is(token.owner_id == claimingAccount, true)

//     let finalBal = await keypomV3.balance();
//     displayBalances(initialBal, finalBal);
// });

test('Lots of Failed Claims', async t => {
    const {funder, keypomV3, nftContract, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_to_balance',
        args: {},
        attachedDeposit: NEAR.parse("10").toString(),
        shouldLog: false
    })

    const nftAsset1 = {
        nft_contract_id: nftContract.accountId
    }

    const dropId = "drop-id";
    const asset_data = [
        {
        assets: [nftAsset1, nftAsset1, nftAsset1, nftAsset1, 
            nftAsset1, nftAsset1, nftAsset1],
            uses: 1
        },
        {
            assets: [null],
                uses: 1
            }
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

    let token_ids: string[] = []

    // This token exists!!
    await functionCall({
        signer: funder,
        receiver: nftContract,
        methodName: 'nft_transfer_call',
        args: {
            receiver_id: keypomV3.accountId,
            token_id: "token1",
            msg: dropId
        },
        attachedDeposit: "1"
    })

    token_ids.push("token1")


    // None of these tokens exist
    for (let i = 0; i < 5; i++) {
        await functionCall({
            signer: nftContract,
            receiver: keypomV3,
            methodName: 'nft_on_transfer',
            args: {
                sender_id: funder.accountId,
                token_id: `token${i+2}`,
                msg: dropId
            }
        })

        token_ids.push(`token${i+2}`)
    }

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract.accountId,
            token_ids
        }],
    })

    // First transfer should succeed, next 5 should fail because token doesn't exist, the last should fail because there aren't enough tokens sent
    // Claim drop
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        shouldPanic: true
    })
    t.is(result.response, "false")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId
    t.is(await (keypomV3.getAccount(claimingAccount)).exists(), true);

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), true)
    t.is(await doesDropExist(keypomV3, dropId), true)

    let token: {token_id: string, owner_id: string} = await nftContract.view('nft_token', {token_id: token_ids[0]});
    t.is(token.owner_id == claimingAccount, true)

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract.accountId,
            token_ids: token_ids.slice(1, -1)
        }],
    })

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});