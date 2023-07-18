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
test('Ensuring drop creation gas limit is accurate', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 1;
    let keyPairs = await generateKeyPairs(numKeys + 1);

    const nftAsset1 = {
        nft_contract_id: nftContract.accountId
    }

    const assets_per_use = {
        1: [nftAsset1, nftAsset1, nftAsset1, nftAsset1, nftAsset1, 
            nftAsset1, nftAsset1, nftAsset1, nftAsset1, nftAsset1, 
            nftAsset1, nftAsset1, nftAsset1, nftAsset1, nftAsset1,
            nftAsset1, nftAsset1, nftAsset1, nftAsset1, nftAsset1]
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_per_use,
            public_keys: keyPairs.publicKeys.slice(0, -1)
        },
        attachedDeposit: NEAR.parse("10").toString(),
    })

    // 12TGas for NFT asset, 20 tokens + other gas = 293TGas
    let tokenIds: string[]= [];
    let numTokens = 20;
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

    console.log(`new PK: ${keyPairs.publicKeys[-1]}`)
    // CAAC
    await claimWithRequiredGas({
        keypomV3,
        root,
        key: keyPairs.keys[0],
        publicKey: keyPairs.publicKeys[0],
        // createAccount: true,
        // newPublicKey: keyPairs.publicKeys[keyPairs.publicKeys.length - 1]
    })

    // After a succesful claim, Keypom keys should be back to just the one FAK
    let keypomKeys = await keypomV3.viewAccessKeys(keypomV3.accountId);
    t.is(keypomKeys.keys.length, 1);

});

// test('Hex ID', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 1;
//     let keyPairs = await generateKeyPairs(numKeys);

//     const assets_per_use = {
//         1: [null]
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

//     // Claim
//     let hexedAccount = await claimWithRequiredGas({
//         keypomV3,
//         root,
//         key: keyPairs.keys[0],
//         publicKey: keyPairs.publicKeys[0],
//     })

//     t.is(hexedAccount, "98793cd91a3f870fb126f66285808c7e094afcfc4eda8a970f6648cdf0dbd6de");

// });

// Lots of Keys
// test('Bunch of Keys', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "drop-id";
//     // must be multiple of 100
//     const numKeys = 10000;
//     let keyPairs = await generateKeyPairs(numKeys);
    
//     const assets_per_use = {
//         1: [null],
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: []
//         },
//         attachedDeposit: NEAR.parse("0.01").toString(),
//     })

//     let keysAdded = 0
//     while(keysAdded < numKeys){
//         let keyPairs = await generateKeyPairs(100);
//         // PK already exists on the contract
//         await functionCall({
//             signer: funder,
//             receiver: keypomV3,
//             methodName: 'add_keys',
//             args: {
//                 drop_id: dropId,
//                 public_keys: keyPairs.publicKeys
//             },
//             attachedDeposit: NEAR.parse("1.4518865").toString()
//         })
//         keysAdded += 100;
//         console.log(`Keys added: ${keysAdded}`)
//     }
    

//     // Keys should not have changed from last time since drop creation has failed
//     let keypomKeys2 = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys2.keys.length, numKeys + 1);
// });

// Empty
// test('Empty', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2} = t.context.accounts;
//     let initialBal = await keypomV3.balance();

//     const dropId = "drop-id";
    
//     const assets_per_use = {
//         1: [null],
//     }
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             assets_per_use,
//             public_keys: []
//         },
//         attachedDeposit: NEAR.parse("0.01").toString(),
//     })
    
//     // Keys should not have changed from last time since drop creation has failed
//     let keypomKeys2 = await keypomV3.viewAccessKeys(keypomV3.accountId);
//     t.is(keypomKeys2.keys.length, 0);
// });

