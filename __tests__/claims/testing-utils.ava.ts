import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, InternalNFTData } from "../utils/types";
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
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"} });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });
    // await nftContract2.call(nftContract2, 'new_default_meta', { owner_id: nftContract2 });


    const ftContract1 = await root.createSubAccount('ft_contract_1');
    // const ftContract2 = await root.createSubAccount('ft_contract_2');

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
test('CAAC', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    const nearAsset1 = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_for_all_uses = {
        assets: [null],
        num_uses: 1
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data_for_all_uses,
            public_keys: [keyPairs.publicKeys[0]]
        },
    })

    t.is(await doesDropExist(keypomV3, dropId), false)

    // CAAC using just createAccount flag
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
    })

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)
});

test('Force CAAC Failure', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    const nearAsset1 = {
        yoctonear: NEAR.parse("1").toString()
    }

    const assets_per_use = {
        1: [null]
    }

    const asset_data_for_all_uses = {
        assets: [null],
        num_uses: 1
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data_for_all_uses,
            public_keys: [keyPairs.publicKeys[0]]
        },
    })

    t.is(await doesDropExist(keypomV3, dropId), false)

    // CAAC using just createAccount flag
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        receiverId: ali.accountId,
        shouldPanic: true
    })

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)
});

test('Claim Normal', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    const nearAsset1 = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_for_all_uses = {
        assets: [null],
        num_uses: 1
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data_for_all_uses,
            public_keys: [keyPairs.publicKeys[0]]
        },
    })

    t.is(await doesDropExist(keypomV3, dropId), false)

    // CAAC using just createAccount flag
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        receiverId: ali.accountId
    })

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)
});

test('Claim Implicit', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    const nearAsset1 = {
        yoctonear: NEAR.parse("1").toString()
    }

    const assets_per_use = {
        1: [nearAsset1]
    }

    const asset_data_for_all_uses = {
        assets: [null],
        num_uses: 1
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data_for_all_uses,
            public_keys: [keyPairs.publicKeys[0]]
        },
    })

    t.is(await doesDropExist(keypomV3, dropId), false)

    // Implicit claimflags
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        useImplicitAccount: true
    })

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)
});