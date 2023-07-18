import anyTest, { TestFn } from "ava";
import { claimTrialAccountDrop, createDrop, createTrialAccountDrop, getDrops, getUserBalance, parseNearAmount, trialCallMethod } from "keypom-js";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
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

    
    await keypomV3.deploy(`./out/mapping.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    const nftContract2 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);

    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });
    await nftContract2.call(nftContract2, 'new_default_meta', { owner_id: nftContract2 });


    // const ftContract1 = await root.createSubAccount('ft_contract_1');
    // const ftContract2 = await root.createSubAccount('ft_contract_2');


    // await ftContract1.deploy(`./__tests__/ext-wasm/ft.wasm`);
    // await ftContract2.deploy(`./__tests__/ext-wasm/ft.wasm`);

    
    // await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });
    // await ftContract2.call(ftContract2, 'new_default_meta', { owner_id: ftContract2, total_supply: totalSupply.toString() });


    // Deposit storage
    // await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})


    // Send FTs
    // await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})


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

    await functionCall({
        signer: funder,
        receiver: nftContract2,
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
    t.context.accounts = { root, funder, keypomV3, nftContract, nftContract2,  };
    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Multi-assets 
// account creation failed -> should refund ALL assets & key is decremented
test('Account Creation Fail in CAAC', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    const nftAsset1 = {
        nft_contract_id: nftContract.accountId
    }

    const assets_per_use = {
        1: [nftAsset1],
        2: [null]
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
        attachedDeposit: NEAR.parse("1").toString(),
    })

    // 12TGas for NFT asset, 20 tokens + other gas = 293TGas
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

    // Assert Assets
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract.accountId,
            token_ids: tokenIds
        },],
    })

    // let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    // t.is(keyInfo.uses_remaining, 2)

    let response = await claimWithRequiredGas({
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

    let token: {token_id: string, owner_id: string} = await nftContract.view('nft_token', {token_id: tokenIds[0]});
    console.log(`${token.token_id} is owned by ${token.owner_id}`)

    let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 2)
    // let keyInfo2: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    // t.is(keyInfo2.uses_remaining, 1)

    // let nftTokens: Array<{owner_id: string, token_id: string}> = await nftContract.view('nft_tokens_for_owner', {account_id: funder.accountId});
    // console.log(nftTokens)
    // t.is(nftTokens.includes({owner_id: funder.accountId, token_id: tokenIds[0]}), true)

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
