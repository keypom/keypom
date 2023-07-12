import anyTest, { TestFn } from "ava";
import { claimTrialAccountDrop, createDrop, createTrialAccountDrop, getDrops, getUserBalance, parseNearAmount, trialCallMethod } from "keypom-js";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, displayBalances, functionCall, generateKeyPairs, initKeypomConnection } from "../utils/general";
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
    
    await keypomV3.deploy(`./out/mapping.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });

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

test('Delete a lot of NFTs with some invalid by passing in token IDs', async t => {
    const {funder, keypomV3, nftContract, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nftAsset1 = {
        nft_contract_id: nftContract.accountId
    }

    const dropId = "drop-id";
    const assets_per_use = {
        1: [nftAsset1]
    }

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_per_use,
            public_keys: [],
        },
        attachedDeposit: NEAR.parse("10").toString()
    })

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

    // loop 20 times
    let tokenIds = ["token1"]
    let numTokens = 3;
    for (let i = 0; i < numTokens; i++) {
        tokenIds.push(`token1${i}`);
        await functionCall({
            signer: nftContract,
            receiver: keypomV3,
            methodName: 'nft_on_transfer',
            args: {
                sender_id: funder.accountId,
                token_id: `token1${i}`,
                msg: dropId
            }
        })
    }

    let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log(`dropInfo: ${JSON.stringify(dropInfo)}`)
    t.is(dropInfo.internal_assets_data.length, 1);
    t.is((dropInfo.internal_assets_data[0] as InternalNFTData).nft.token_ids.length, numTokens + 1);

    let response = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'withdraw_nft_tokens',
        args: {
            drop_id: dropId,
            nft_contract_id: nftContract.accountId,
            token_ids: tokenIds
        },
        shouldPanic: true
    })
    console.log('response: ', response)
    t.is(response, "false");

    dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log(`dropInfo: ${JSON.stringify(dropInfo)}`)
    t.is(dropInfo.internal_assets_data.length, 1);
    t.is((dropInfo.internal_assets_data[0] as InternalNFTData).nft.token_ids.length, numTokens + 1);

    // None of the tokens should have been transferred
    let tokensForOwner = await nftContract.view('nft_supply_for_owner', {account_id: funder.accountId});
    console.log('tokensForOwner: ', tokensForOwner)
    t.is(tokensForOwner, '0');

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});

test('Delete NFTs, some invalid using limit', async t => {
    const {funder, keypomV3, nftContract, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nftAsset1 = {
        nft_contract_id: nftContract.accountId
    }

    const dropId = "drop-id";
    const assets_per_use = {
        1: [nftAsset1]
    }

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            assets_per_use,
            public_keys: [],
        },
        attachedDeposit: NEAR.parse("10").toString()
    })

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

    // loop 20 times
    let numTokens = 3;
    for (let i = 0; i < numTokens; i++) {
        await functionCall({
            signer: nftContract,
            receiver: keypomV3,
            methodName: 'nft_on_transfer',
            args: {
                sender_id: funder.accountId,
                token_id: `token1${i}`,
                msg: dropId
            }
        })
    }

    let dropInfo: ExtDrop = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log(`dropInfo: ${JSON.stringify(dropInfo)}`)
    t.is(dropInfo.internal_assets_data.length, 1);
    t.is((dropInfo.internal_assets_data[0] as InternalNFTData).nft.token_ids.length, numTokens + 1);

    let response = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'withdraw_nft_tokens',
        args: {
            drop_id: dropId,
            nft_contract_id: nftContract.accountId
        },
        shouldPanic: true
    })
    console.log('response: ', response)
    t.is(response, "false");

    dropInfo = await keypomV3.view('get_drop_information', {drop_id: dropId});
    console.log(`dropInfo: ${JSON.stringify(dropInfo)}`)
    t.is(dropInfo.internal_assets_data.length, 1);
    t.is((dropInfo.internal_assets_data[0] as InternalNFTData).nft.token_ids.length, numTokens + 1);

    // None of the tokens should have been transferred
    let tokensForOwner = await nftContract.view('nft_supply_for_owner', {account_id: funder.accountId});
    console.log('tokensForOwner: ', tokensForOwner)
    t.is(tokensForOwner, '0');

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
});