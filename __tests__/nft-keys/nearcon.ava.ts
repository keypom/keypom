import anyTest, { TestFn } from "ava";
import { KeyPair, NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, getDropInformation, getKeyInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, JsonKeyInfo, NFTTokenObject, SimpleData, TokenMetadata } from "../utils/types";
import { BN } from "bn.js";

import { createDropArgs, generatePerUsePasswords, PasswordPerUse, wrapTxnParamsForTrial } from "./utils";


const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    keypairs: {keys: KeyPair[], publicKeys: string[]}
  }>;

  test.beforeEach(async (t) => {
    console.log("Starting test");
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy all 2 contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    console.log("Deployed contracts");
    
    // Init empty/default linkdrop contract
    await root.call(root, 'new', {});
    //init new keypom contract and setting keypom as the owner. 
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    
    console.log("Initialized contracts");
    // Test users
    const funder = await root.createSubAccount('funder');
    const bob = await root.createSubAccount('bob');

    // Add 10k $NEAR to owner's account
    await funder.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

    //add 2NEAR to ali's keypom balance
    await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("5000").toString()});
        
    const keys = await generateKeyPairs(50);
    const basePassword = "MASTER_KEY"

    let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
        publicKeys: keys.publicKeys,
        basePassword,
        uses: [1]
    });

    //create a drop with Ali, doesn't front any cost.
    await funder.call(keypom, 'create_drop', createDropArgs({pubKeys: keys.publicKeys, passwords, root, wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), {gas: '300000000000000'});

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, funder, bob };
    t.context.keypairs = keys;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Transfer NFT Using Keypom', async t => {
    const { keypom, funder, bob, root } = t.context.accounts;
    const keys = t.context.keypairs;

    const newKeys = await generateKeyPairs(1);
    
    let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, keys.publicKeys[0]);
    let initialAllowance = keyInfo.allowance;
    console.log('keyInfo before: ', keyInfo)
    
    await keypom.setKey(keys.keys[0]);
    await keypom.call(keypom, 'nft_transfer', {token_id: `0:0`, receiver_id: funder.accountId, memo: newKeys.publicKeys[0]});
    try {
        await keypom.call(keypom, 'nft_transfer', {token_id: `0:0`, receiver_id: bob.accountId, memo: keys.publicKeys[0]});
        keyInfo = await getKeyInformation(keypom, keys.publicKeys[0]);
        t.is(1, 2);
    } catch (e) {
        t.is(1, 1);
    }

    keyInfo = await getKeyInformation(keypom, newKeys.publicKeys[0]);
    console.log('keyInfo after: ', keyInfo)
    t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
    t.is(keyInfo.owner_id, funder.accountId);
});

test('Transfer NFT Using Owner Account', async t => {
    const { keypom, funder, bob, root } = t.context.accounts;
    const keyInfos = t.context.keypairs;

    const newKeys = await generateKeyPairs(1);
    
    await keypom.setKey(keyInfos.keys[0]);
    await keypom.call(keypom, 'nft_transfer', {receiver_id: funder.accountId, memo: newKeys.publicKeys[0]});
    
    let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, newKeys.publicKeys[0]);
    let initialAllowance = keyInfo.allowance;
    console.log('keyInfo before: ', keyInfo)

    await funder.call(keypom, 'nft_transfer', {token_id: `0:0`, receiver_id: bob.accountId, memo: keyInfos.publicKeys[0]});

    keyInfo = await getKeyInformation(keypom, keyInfos.publicKeys[0]);
    console.log('keyInfo after: ', keyInfo)
    t.assert(new BN(initialAllowance).eq(new BN(keyInfo.allowance)));
    t.is(keyInfo.owner_id, bob.accountId);
});

test('Transfer NFT Not Owned By Account', async t => {
    const { keypom, funder, bob, root } = t.context.accounts;
    const keyInfos = t.context.keypairs;

    const newKeys = await generateKeyPairs(1);

    await keypom.setKey(keyInfos.keys[0]);
    await keypom.call(keypom, 'nft_transfer', {receiver_id: funder.accountId, memo: newKeys.publicKeys[0]});
    
    let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, newKeys.publicKeys[0]);
    let initialAllowance = keyInfo.allowance;
    console.log('keyInfo before: ', keyInfo)

    try {
        await bob.call(keypom, 'nft_transfer', {token_id: `0:0`, receiver_id: bob.accountId, memo: keyInfos.publicKeys[0]});
        t.is(1, 2);
    } catch (e) {
        t.is(1, 1);
    }

    keyInfo = await getKeyInformation(keypom, newKeys.publicKeys[0]);
    console.log('keyInfo after: ', keyInfo)
    t.assert(new BN(initialAllowance).eq(new BN(keyInfo.allowance)));
    t.is(keyInfo.owner_id, funder.accountId);
});