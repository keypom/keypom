import anyTest, { TestFn } from "ava";
import { formatNearAmount } from "near-api-js/lib/utils/format";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, InternalNFTData } from "../utils/types";
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
    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"} });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract});

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
        receiver: keypomV3,
        methodName: 'add_to_balance',
        args: {},
        attachedDeposit: NEAR.parse("10").toString(),
        shouldLog: false
    })
    
    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, keypomV3, nftContract };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Simple NFT Creation', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nftAsset1 = {
        nft_contract_id: "foobar.test.near"
    }

    const dropId = "drop-id";
    const asset_data = [
        {
            assets: [nftAsset1],
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
            }],
        },
        attachedDeposit: NEAR.parse("10").toString()
    })

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: "foobar.test.near",
            token_ids: []
        }],
    })

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Add NFTs to created drop', async t => {
    const {funder, keypomV3, nftContract, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nftAsset1 = {
        nft_contract_id: nftContract.accountId
    }

    const dropId = "drop-id";
    const asset_data = [
        {
            assets: [nftAsset1],
            uses: 1
        }
    ]

    let userBal1: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
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
            }],
        },
        attachedDeposit: NEAR.parse("10").toString()
    })

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

    let userBal2: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
    let balChange = formatNearAmount((BigInt(userBal1) - BigInt(userBal2)).toString(), 5);

    t.is(balChange > "0", true)

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract.accountId,
            token_ids: ["token1"]
        }],
    })

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Add a Ton of NFTs', async t => {
    const {funder, keypomV3, nftContract, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nftAsset1 = {
        nft_contract_id: nftContract.accountId
    }

    const dropId = "drop-id";
    const asset_data = [{
        assets: [nftAsset1],
        uses: 1
    }]

    let userBal1: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});

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
            }],
        },
        attachedDeposit: NEAR.parse("10").toString()
    })

    let tokenIds: string[] = []
    // loop 50 times
    for (let i = 0; i < 50; i++) {
        await functionCall({
            signer: nftContract,
            receiver: keypomV3,
            methodName: 'nft_on_transfer',
            args: {
                sender_id: funder.accountId,
                token_id: `token${i}`,
                msg: dropId
            }
        })
        tokenIds.push(`token${i}`)
    }

    let userBal2: string = await keypomV3.view('get_user_balance', {account_id: funder.accountId});
    let balChange = formatNearAmount((BigInt(userBal1) - BigInt(userBal2)).toString(), 5);

    t.is(balChange > "0", true)

    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 1)

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract.accountId,
            token_ids: tokenIds
        }],
    })

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});