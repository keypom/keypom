import anyTest, { TestFn } from "ava";
import { KeyPair, NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, getDropInformation, getKeyInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, JsonDrop, JsonKeyInfo, NFTTokenObject, SimpleData, TokenMetadata } from "../utils/types";
import { BN, min } from "bn.js";

import { createDropArgs, generatePerUsePasswords, ListingJson, PasswordPerUse, wrapTxnParamsForTrial } from "./utils";

let actualContracts = Array(10).fill('nft-v2.keypom.near');
let actualAmounts = Array(10).fill(NEAR.parse("100").toString());
let actualMethods = Array(10).fill('create_account_and_claim');
let attachedDeposit = NEAR.parse("1").toString();
let extraAllowance = NEAR.parse("0.1").toString();

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

    // Deploy Contracts
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    
    const keypom = await root.createSubAccount('keypom');
    await keypom.deploy(`./out/keypom.wasm`);

    const mintbase = await root.createSubAccount('mintbase');
    await mintbase.deploy(`./__tests__/ext-wasm/mintbase-old.wasm`);
    
    console.log("Deployed contracts");
    
    // Initialize Contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    await mintbase.call(mintbase, 'init', { owner: mintbase, mintbase_cut: 0, fallback_cut: 0, listing_lock_seconds: "0" });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });
    console.log("Initialized contracts");
    
    // Deposit storage on mintbase contract
    await keypom.call(mintbase, 'deposit_storage', {},{attachedDeposit: NEAR.parse("10").toString()});

    // Test users
    const funder = await root.createSubAccount('funder');
    const bob = await root.createSubAccount('bob');

    // Add 10k $NEAR to owner's account and add 5k to their keypom balance
    await funder.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("5000").toString()});

    const keys = await generateKeyPairs(1);
    const basePassword = "MASTER_KEY"

    let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
        publicKeys: keys.publicKeys,
        basePassword,
        uses: [1]
    });

    // Create ticket drop
    await funder.call(keypom, 'create_drop', createDropArgs({
        pubKeys: keys.publicKeys, 
        passwords, 
        root, 
        actualContracts,
        actualMethods,
        actualAmounts,
        attachedDeposit,
        extraAllowance,
        wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), 
        {gas: '300000000000000'}
    );

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, funder, bob, mintbase, nftContract };
    t.context.keypairs = keys;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Test Migration', async t => {
    const { keypom, funder, mintbase, bob, root, nftContract } = t.context.accounts;
    const sellerKeys = t.context.keypairs;

    // Funder deposits storage on mintbase contract since they own initial NFT
    await funder.call(mintbase, 'deposit_storage', {},{attachedDeposit: NEAR.parse("10").toString()});

    // Mint and put up an NFT for sale on legacy market
    let mintbase_args = JSON.stringify({
        price: NEAR.parse('1').toString()
    })
    await funder.call(nftContract, 'nft_mint', {token_id: `migration_token`, receiver_id: funder.accountId, metadata: {title: "hello", description: "hello"}}, {attachedDeposit: NEAR.parse("1").toString()});
    await funder.call(nftContract, 'nft_approve', {token_id: `migration_token`, account_id: mintbase.accountId, msg: mintbase_args}, {attachedDeposit: NEAR.parse("1").toString()});

    // Ensure the listing is correct
    let listing: ListingJson = await mintbase.view('get_listing', {nft_contract_id: nftContract, token_id: `migration_token`});
    t.assert(listing.nft_token_id === `migration_token`);
    t.assert(listing.price === NEAR.parse('1').toString());
    t.assert(listing.nft_owner_id === funder.accountId);
    t.assert(listing.nft_contract_id === nftContract.accountId);
    t.assert(listing.currency === 'near');
    t.assert(listing.nft_approval_id === 0);

    //Deploy the new version of the mintbase contract and check for state conflicts
    await mintbase.deploy(`./__tests__/ext-wasm/mintbase-new.wasm`);
    try {
        listing = await mintbase.view('get_listing', {nft_contract_id: nftContract, token_id: `migration_token`});
        t.fail('Should have thrown an error');
    } catch {
        t.pass();
    }

    // After migrating state, things should work well.
    await mintbase.call(mintbase, 'migrate', {keypom_contract_root: keypom.accountId});

    listing = await mintbase.view('get_listing', {nft_contract_id: nftContract, token_id: `migration_token`});
    t.assert(listing.nft_token_id === `migration_token`);
    t.assert(listing.price === NEAR.parse('1').toString());
    t.assert(listing.nft_owner_id === funder.accountId);
    t.assert(listing.nft_contract_id === nftContract.accountId);
    t.assert(listing.currency === 'near');
    t.assert(listing.nft_approval_id === 0);

    // Now with migration out of the way, we can test the new mintbase contract and sell access keys
    let initialAllowance = (await getKeyInformation(keypom, sellerKeys.publicKeys[0])).allowance;
    
    // Put the first key up for sale (owner does NOT have a wallet here)
    await keypom.setKey(sellerKeys.keys[0]);
    let new_mintbase_args = JSON.stringify({
        price: NEAR.parse('1').toString(),
        owner_pub_key: sellerKeys.publicKeys[0]
    })
    await keypom.call(keypom, 'nft_approve', {account_id: mintbase.accountId, msg: new_mintbase_args});
    listing = await mintbase.view('get_listing', {nft_contract_id: keypom, token_id: `0:0`});
    t.assert(listing.nft_token_id === `0:0`);
    t.assert(listing.price === NEAR.parse('1').toString());
    t.assert(listing.nft_owner_id === keypom.accountId);
    t.assert(listing.nft_contract_id === keypom.accountId);
    t.assert(listing.currency === 'near');
    t.assert(listing.nft_approval_id === 0);

    // After key is put for sale, its allowance should have decremented
    let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, sellerKeys.publicKeys[0]);
    t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
    t.assert(keyInfo.approved_account_ids[mintbase.accountId] === 0);
    t.is(keyInfo.next_approval_id, 1);
    initialAllowance = keyInfo.allowance;

    /// Bob purchases the key
    const buyerKeys = await generateKeyPairs(1);
    await bob.call(mintbase, 'buy', {nft_contract_id: keypom.accountId, token_id: `0:0`, new_pub_key: buyerKeys.publicKeys[0]}, {attachedDeposit: NEAR.parse('1').toString(), gas: '300000000000000'});

    // Now that bob bought the key, his key should have the same allowance as what seller left off with and should have all remaining uses
    keyInfo = await getKeyInformation(keypom, buyerKeys.publicKeys[0]);
    t.is(keyInfo.owner_id, bob.accountId);
    t.is(keyInfo.allowance, initialAllowance)
    t.is(keyInfo.remaining_uses, 2);

    // Seller should now have a simple $NEAR drop with 0.05 $NEAR less than the 1 $NEAR purchase price
    let sellerNewDrop: JsonDrop = await keypom.view('get_drop_information', {key: sellerKeys.publicKeys[0]});
    t.is(sellerNewDrop.deposit_per_use, NEAR.parse('0.95').toString());
    t.is(sellerNewDrop.fc, undefined);
    t.is(sellerNewDrop.ft, undefined);
    t.is(sellerNewDrop.nft, undefined);
    t.assert(sellerNewDrop.simple !== undefined);

    // Seller now uses their key to create a new account
    await keypom.setKey(sellerKeys.keys[0]);
    await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `seller.${root.accountId}`, new_public_key: sellerKeys.publicKeys[0]}, {gas: WALLET_GAS});

    // Now that the new account was created, the seller's original key should have been deleted from keypom contract
    try {
        sellerNewDrop = await keypom.view('get_drop_information', {key: sellerKeys.publicKeys[0]});
        t.fail('Should have thrown an error');
    } catch {
        t.pass();
    }

    // Check if seller's account was successfully created
    const sellerAccount = await root.getAccount(`seller.${root.accountId}`);
    console.log('sellerAccount: ', sellerAccount)
    t.assert(sellerAccount !== undefined);
});