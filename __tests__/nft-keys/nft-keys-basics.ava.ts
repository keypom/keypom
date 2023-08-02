import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNFTData, InternalNFTData, InternalFTData, ExtNearData } from "../utils/types";
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

    const nftContract1 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    await nftContract1.call(nftContract1, 'new_default_meta', { owner_id: nftContract1});

    const nftContract2 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    await nftContract2.call(nftContract2, 'new_default_meta', { owner_id: nftContract2});
    
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
    t.context.accounts = { root, funder, keypomV3, nftContract1, nftContract2, ali, bob };

    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Sending really long token IDs
// Sending a TON of token IDs
// Sending tokens to a non NFT drop
// Sending tokens to the wrong drop


// Ensure user is being charged and balance decrements
// Ensure tokens are being added to drop info
// Ensure that if the internal vector gets really long for token IDs, there isn’t a gas hit (measure gas for the case of a ton of token IDs)

// test('Ownerless Keys', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali} = t.context.accounts;
    
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     // ******************* Creating Drop *******************
//     const nearAsset1: ExtNearData = {
//         yoctonear: NEAR.parse("1").toString()
//     }

//     const asset_data_per_use = {
//         assets: [nearAsset1],
//         num_uses: 1
//     };
    
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
//             drop_data: {
//                 nft_keys_config: {
//                     token_metadata: {
//                         title: "my token",
//                         description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
//                         media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
//                     },
//                 }
//             }
//         },
//     }) 

//     let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
//     t.is(found_key_info.owner_id == keypomV3.accountId, true)

//     let found_nft_info: {metadata: {title: string}} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
//     t.is(found_nft_info.metadata.title == "my token", true)
// });

// test('Owned Keys', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali} = t.context.accounts;
    
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     // ******************* Creating Drop *******************
//     const nearAsset1: ExtNearData = {
//         yoctonear: NEAR.parse("1").toString()
//     }

//     const asset_data_per_use = {
//         assets: [nearAsset1],
//         num_uses: 1
//     };
    
//     await functionCall({
//         signer: funder,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: asset_data_per_use,
//             key_data: [{
//                 public_key: keyPairs.publicKeys[0],
//                 key_owner: funder
//             }],
//             drop_data: {
//                 nft_keys_config: {
//                     token_metadata: {
//                         title: "my token",
//                         description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
//                         media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
//                     },
//                 }
//             }
//         },
//     }) 

//     let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
//     t.is(found_key_info.owner_id == funder.accountId, true)

//     let found_nft_info: {metadata: {title: string}} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
//     t.is(found_nft_info.metadata.title == "my token", true)
// });

test('Owned key transfer, approval, revoking - with owner account', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* SETUP *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = {
        assets: [nearAsset1],
        num_uses: 1
    };
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
                key_owner: funder
            }],
            drop_data: {
                nft_keys_config: {
                    token_metadata: {
                        title: "my token",
                        description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
                        media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
                    },
                }
            }
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    t.is(found_key_info.owner_id == funder.accountId, true)

    // add Ali to approved list of transferers
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "nft_approve",
        args:{
            token_id: found_key_info.token_id,
            account_id: ali.accountId
        }
    })
    t.is(found_key_info.owner_id == funder.accountId, true)
    
    console.log("OWNER INFO FROM NFT")
    let found_nft_info: {owner_id: string} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
    console.log(found_nft_info.owner_id)

    // Currently not working, trying to transfer with funder
    // Problem 1: im trying to send with ali, not funder
    let newKeyPair = await generateKeyPairs(1);
    await functionCall({
        signer: ali,
        receiver: keypomV3,
        methodName: "nft_transfer",
        args: {
            token_id: found_key_info.token_id,
            receiver_id: bob.accountId,
            memo: newKeyPair.publicKeys[0]
        }
    })

    // let found_nft_info: {owner_id: string} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
    // console.log(found_nft_info.owner_id)
    // t.is(found_nft_info.owner_id == newKeyPair.publicKeys[0], true)
});

// test('Owned key transfer, approval, revoking - with key', async t => {
//     const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, nftContract2, ali} = t.context.accounts;
    
//     let initialBal = await keypomV3.balance();

//     const dropId = "my-drop-id";
//     const wrongDropId = "wrong-drop-id";
//     const numKeys = 2;
//     let keyPairs = await generateKeyPairs(numKeys);

//     // ******************* Creating Drop *******************
//     const nftAsset1: ExtNFTData = {
//         nft_contract_id: nftContract1.accountId
//     }

//     const asset_data = [
//         {
//             assets: [nftAsset1]
//         },
//     ]

//     await functionCall({
//         signer: ali,
//         receiver: keypomV3,
//         methodName: 'create_drop',
//         args: {
//             drop_id: wrongDropId,
//             asset_data,
//             public_keys: [keyPairs.publicKeys[0]]
//         },
//     }) 

//     // ******************* Adding Assets *******************
//     let token_id = `${Date.now().toString().repeat(45)}`

//     let preSendBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
//     try{
//         await functionCall({
//             signer: nftContract1,
//             receiver: keypomV3,
//             methodName: 'nft_on_transfer',
//             args: {
//                 sender_id: funder.accountId,
//                 token_id,
//                 msg: wrongDropId
//             }
//         })
//     }catch(e){

//     }

//     let postSendBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
//     let balChange = formatNearAmount((BigInt(preSendBal) - BigInt(postSendBal)).toString(), 5);
//     console.log(balChange)
//     t.is(balChange == "0", true)

//     await assertKeypomInternalAssets({
//         keypom: keypomV3,
//         dropId: wrongDropId,
//         expectedNftData: [{
//             contract_id: nftContract1.accountId,
//             token_ids: []
//         }],
//     })
// });





