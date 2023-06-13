import anyTest, { TestFn } from "ava";
import { KeyPair, NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, displayFailureLog, generateKeyPairs, getDropInformation, getKeyInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, JsonDrop, JsonKeyInfo, NFTTokenObject, SimpleData, TokenMetadata } from "../utils/types";
import { BN, min } from "bn.js";

import { createDropArgs, generatePerUsePasswords, hashBuf, ListingJson, PasswordPerUse, sellNFT, wrapTxnParamsForTrial } from "./utils";

let actualContracts = Array(10).fill('nft-v2.keypom.near');
let actualAmounts = Array(10).fill(NEAR.parse("100").toString());
let actualMethods = Array(10).fill('create_account_and_claim');
let attachedDeposit = NEAR.parse("1").toString();
let extraAllowance = NEAR.parse("0.1").toString();

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
  }>;

test.beforeEach(async (t) => {
    console.log("Starting test");
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy all 2 contracts
    const keypom = await root.createSubAccount('keypom');
    await keypom.deploy(`./out/keypom.wasm`);

    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const mintbase = await root.createSubAccount('mintbase');
    await mintbase.deploy(`./__tests__/ext-wasm/mintbase-new.wasm`);
    console.log("Deployed contracts");
    
    // Init empty/default linkdrop contract
    await root.call(root, 'new', {});
    //init new keypom contract and setting keypom as the owner. 
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    await mintbase.call(mintbase, 'init', { owner: mintbase, mintbase_cut: 0, fallback_cut: 0, listing_lock_seconds: "0", keypom_contract_root: keypom.accountId });
    await keypom.call(mintbase, 'deposit_storage', {},{attachedDeposit: NEAR.parse("10").toString()});
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

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, funder, bob, mintbase };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// test('Journey 1: New to NEAR Purchases & Attends Conference', async t => {
//     const { keypom, funder, bob, root, mintbase } = t.context.accounts;
//     const ticketHolderKeys = await generateKeyPairs(1);
//     const basePassword = "MASTER_KEY"
//     let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
//         publicKeys: ticketHolderKeys.publicKeys,
//         basePassword,
//         uses: [1]
//     });
//     await funder.call(keypom, 'create_drop', createDropArgs({
//         pubKeys: ticketHolderKeys.publicKeys, 
//         passwords, root, 
//         actualContracts,
//         actualMethods,
//         actualAmounts,
//         attachedDeposit,
//         extraAllowance,
//         wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), 
//         {gas: '300000000000000'}
//     );
    
//     // Ticker Holder is Scanned Into Event
//     await keypom.setKey(ticketHolderKeys.keys[0]);
//     const claimPassword = Buffer.from(await hashBuf(
//         basePassword + ticketHolderKeys.publicKeys[0] + "1".toString()
//     )).toString('hex');

//     await keypom.call(keypom, 'claim', {account_id: "foo", password: claimPassword}, {gas: '150000000000000'});
//     let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, ticketHolderKeys.publicKeys[0]);
//     t.assert(Object.keys(keyInfo.approved_account_ids).length === 0);
//     t.is(keyInfo.next_approval_id, 0);
//     t.is(keyInfo.remaining_uses, 1);
//     t.is(keyInfo.owner_id, keypom.accountId);

//     // After User is Scanned, Trial Account Creation
//     let ticketHolderAccount = await root.getAccount(`ticket-holder.${root.accountId}`);
//     let userFcArgs = {
//         INSERT_NEW_ACCOUNT: ticketHolderAccount.accountId,
//         INSERT_TRIAL_PUBLIC_KEY: ticketHolderKeys.publicKeys[0],
//     };
//     let args = {
//         account_id: ticketHolderAccount.accountId,
//         fc_args: [JSON.stringify(userFcArgs), null]
//     };
//     await keypom.call(keypom, 'claim', args, {gas: '150000000000000'});

//     const rules: {floor: string} = await ticketHolderAccount.view('get_rules');
//     console.log('rules: ', rules)
//     t.is(rules.floor, '0');

//     try {
//         await getKeyInformation(keypom, ticketHolderKeys.publicKeys[0]);
//         t.fail()
//     } catch {
//         t.pass()
//     }
// });

// test('Journey 2: Crypto Native Purchases & Attends Conference', async t => {
//     const { keypom, funder, bob, root, mintbase } = t.context.accounts;
//     const ticketHolderKeys = await generateKeyPairs(1);
//     const basePassword = "MASTER_KEY"
//     let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
//         publicKeys: ticketHolderKeys.publicKeys,
//         basePassword,
//         uses: [1]
//     });
//     await funder.call(keypom, 'create_drop', createDropArgs({
//         pubKeys: ticketHolderKeys.publicKeys, 
//         nftOwners: [funder.accountId], 
//         passwords, 
//         root, 
//         actualContracts,
//         actualMethods,
//         actualAmounts,
//         attachedDeposit,
//         extraAllowance,
//         wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), 
//         {gas: '300000000000000'}
//     );
    
//     // Ticker Holder is Scanned Into Event
//     await keypom.setKey(ticketHolderKeys.keys[0]);
//     const claimPassword = Buffer.from(await hashBuf(
//         basePassword + ticketHolderKeys.publicKeys[0] + "1".toString()
//     )).toString('hex');

//     await keypom.call(keypom, 'claim', {account_id: "foo", password: claimPassword}, {gas: '150000000000000'});
//     let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, ticketHolderKeys.publicKeys[0]);
//     t.assert(Object.keys(keyInfo.approved_account_ids).length === 0);
//     t.is(keyInfo.next_approval_id, 0);
//     t.is(keyInfo.remaining_uses, 1);
//     t.is(keyInfo.owner_id, funder.accountId);

//     // After User is Scanned, Trial Account Creation
//     let ticketHolderAccount = await root.getAccount(`ticket-holder.${root.accountId}`);
//     let userFcArgs = {
//         INSERT_NEW_ACCOUNT: ticketHolderAccount.accountId,
//         INSERT_TRIAL_PUBLIC_KEY: ticketHolderKeys.publicKeys[0],
//     };
//     let args = {
//         account_id: ticketHolderAccount.accountId,
//         fc_args: [JSON.stringify(userFcArgs), null]
//     };
//     await keypom.call(keypom, 'claim', args, {gas: '150000000000000'});

//     const rules: {floor: string} = await ticketHolderAccount.view('get_rules');
//     console.log('rules: ', rules)
//     t.is(rules.floor, '0');

//     try {
//         await getKeyInformation(keypom, ticketHolderKeys.publicKeys[0]);
//         t.fail()
//     } catch {
//         t.pass()
//     }
// });

// test('Journey 3: New to NEAR Purchases & Sells on Secondary Marketplace', async t => {
//     const { keypom, funder, bob, root, mintbase } = t.context.accounts;

//     const sellerKeys = await generateKeyPairs(1);
//     const basePassword = "MASTER_KEY"
//     let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
//         publicKeys: sellerKeys.publicKeys,
//         basePassword,
//         uses: [1]
//     });
//     await funder.call(keypom, 'create_drop', createDropArgs({
//         pubKeys: sellerKeys.publicKeys, 
//         passwords, 
//         root, 
//         actualContracts,
//         actualMethods,
//         actualAmounts,
//         attachedDeposit,
//         extraAllowance,
//         wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), 
//         {gas: '300000000000000'}
//     );
    
//     // Now with migration out of the way, we can test the new mintbase contract and sell access keys
//     let initialAllowance = (await getKeyInformation(keypom, sellerKeys.publicKeys[0])).allowance;
    
//     // Put the first key up for sale (owner does NOT have a wallet here)
//     await keypom.setKey(sellerKeys.keys[0]);
//     let new_mintbase_args = JSON.stringify({
//         price: NEAR.parse('1').toString(),
//         owner_pub_key: sellerKeys.publicKeys[0]
//     })
//     await keypom.call(keypom, 'nft_approve', {account_id: mintbase.accountId, msg: new_mintbase_args});
//     let listing: ListingJson = await mintbase.view('get_listing', {nft_contract_id: keypom, token_id: `0:0`});
//     t.assert(listing.nft_token_id === `0:0`);
//     t.assert(listing.price === NEAR.parse('1').toString());
//     t.assert(listing.nft_owner_id === keypom.accountId);
//     t.assert(listing.nft_contract_id === keypom.accountId);
//     t.assert(listing.currency === 'near');
//     t.assert(listing.nft_approval_id === 0);

//     // After key is put for sale, its allowance should have decremented
//     let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, sellerKeys.publicKeys[0]);
//     t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
//     t.assert(keyInfo.approved_account_ids[mintbase.accountId] === 0);
//     t.is(keyInfo.next_approval_id, 1);
//     initialAllowance = keyInfo.allowance;

//     /// Bob purchases the key
//     const buyerKeys = await generateKeyPairs(1);
//     await bob.call(mintbase, 'buy', {nft_contract_id: keypom.accountId, token_id: `0:0`, new_pub_key: buyerKeys.publicKeys[0]}, {attachedDeposit: NEAR.parse('1').toString(), gas: '300000000000000'});

//     // Now that bob bought the key, his key should have the same allowance as what seller left off with and should have all remaining uses
//     keyInfo = await getKeyInformation(keypom, buyerKeys.publicKeys[0]);
//     t.is(keyInfo.owner_id, bob.accountId);
//     t.is(keyInfo.allowance, initialAllowance)
//     t.is(keyInfo.remaining_uses, 2);

//     // Seller should now have a simple $NEAR drop with 0.05 $NEAR less than the 1 $NEAR purchase price
//     let sellerNewDrop: JsonDrop = await keypom.view('get_drop_information', {key: sellerKeys.publicKeys[0]});
//     t.is(sellerNewDrop.deposit_per_use, NEAR.parse('0.95').toString());
//     t.is(sellerNewDrop.fc, undefined);
//     t.is(sellerNewDrop.ft, undefined);
//     t.is(sellerNewDrop.nft, undefined);
//     t.assert(sellerNewDrop.simple !== undefined);

//     // Seller now uses their key to create a new account
//     await keypom.setKey(sellerKeys.keys[0]);
//     await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `seller.${root.accountId}`, new_public_key: sellerKeys.publicKeys[0]}, {gas: WALLET_GAS});

//     // Now that the new account was created, the seller's original key should have been deleted from keypom contract
//     try {
//         sellerNewDrop = await keypom.view('get_drop_information', {key: sellerKeys.publicKeys[0]});
//         t.fail('Should have thrown an error');
//     } catch {
//         t.pass();
//     }

//     // Check if seller's account was successfully created and exists
//     const sellerAccount = await root.getAccount(`seller.${root.accountId}`);
//     const doesExist = await sellerAccount.exists();
//     t.assert(doesExist === true);
// });

// test('Journey 4: Crypto Native Purchases & Sells on Secondary Marketplace', async t => {
//     const { keypom, funder, bob, root, mintbase } = t.context.accounts;
//     const sellerKeys = await generateKeyPairs(1);
//     const basePassword = "MASTER_KEY"
//     let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
//         publicKeys: sellerKeys.publicKeys,
//         basePassword,
//         uses: [1]
//     });
//     await funder.call(keypom, 'create_drop', createDropArgs({
//         pubKeys: sellerKeys.publicKeys, 
//         nftOwners: [funder.accountId], 
//         passwords, 
//         root, 
//         actualContracts,
//         actualMethods,
//         actualAmounts,
//         attachedDeposit,
//         extraAllowance,
//         wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), 
//         {gas: '300000000000000'}
//     );
    
//     // Now with migration out of the way, we can test the new mintbase contract and sell access keys
//     let initialAllowance = (await getKeyInformation(keypom, sellerKeys.publicKeys[0])).allowance;
    
//     // Put the first key up for sale
//     await funder.call(mintbase, 'deposit_storage', {},{attachedDeposit: NEAR.parse("1").toString()});
    
//     // owner_pub_key cannot be used to approve a key if the funder is signing the txn. It can only be used
//     // if Keypom is signing
//     try {
//         let new_mintbase_args = JSON.stringify({
//             price: NEAR.parse('1').toString(),
//             owner_pub_key: sellerKeys.publicKeys[0]
//         })
//         await funder.call(keypom, 'nft_approve', {token_id: `0:0`, account_id: mintbase.accountId, msg: new_mintbase_args});
//         t.fail();
//     } catch (e) {
//         t.pass();
//     }

//     let new_mintbase_args = JSON.stringify({
//         price: NEAR.parse('1').toString()
//     })
//     await funder.call(keypom, 'nft_approve', {token_id: `0:0`, account_id: mintbase.accountId, msg: new_mintbase_args});
    
//     let listing: ListingJson = await mintbase.view('get_listing', {nft_contract_id: keypom, token_id: `0:0`});
//     console.log('listing: ', listing)
//     t.assert(listing.nft_token_id === `0:0`);
//     t.assert(listing.price === NEAR.parse('1').toString());
//     t.assert(listing.nft_owner_id === funder.accountId);
//     t.assert(listing.nft_contract_id === keypom.accountId);
//     t.assert(listing.currency === 'near');
//     t.assert(listing.nft_approval_id === 1);

//     // After key is put for sale, its allowance should be the same
//     let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, sellerKeys.publicKeys[0]);
//     t.assert(new BN(initialAllowance).eq(new BN(keyInfo.allowance)));
//     t.assert(keyInfo.approved_account_ids[mintbase.accountId] === 1);
//     t.is(keyInfo.next_approval_id, 2);
//     initialAllowance = keyInfo.allowance;

//     /// Bob purchases the key
//     const buyerKeys = await generateKeyPairs(1);
//     const funderBalBefore = await funder.availableBalance();
//     console.log('funderBalBefore: ', funderBalBefore.toString());

//     await bob.call(mintbase, 'buy', {nft_contract_id: keypom.accountId, token_id: `0:0`, new_pub_key: buyerKeys.publicKeys[0]}, {attachedDeposit: NEAR.parse('1').toString(), gas: '300000000000000'});

//     const funderBalAfter = await funder.availableBalance();
//     console.log('funderBalAfter: ', funderBalAfter.toString());

//     // Funder should have received 1.1 $NEAR (1 $NEAR for the key and 0.1 $NEAR for the storage deposit)
//     t.is(funderBalAfter.sub(funderBalBefore).toString(), NEAR.parse('1.01').toString());

//     // Now that bob bought the key, his key should have the same allowance as what seller left off with and should have all remaining uses
//     keyInfo = await getKeyInformation(keypom, buyerKeys.publicKeys[0]);
//     t.is(keyInfo.owner_id, bob.accountId);
//     t.is(keyInfo.allowance, initialAllowance)
//     t.is(keyInfo.remaining_uses, 2);

//     // Now that the key was purchased, the seller's original key should have been deleted from keypom contract
//     try {
//         await keypom.view('get_drop_information', {key: sellerKeys.publicKeys[0]});
//         t.fail('Should have thrown an error');
//     } catch {
//         t.pass();
//     }
// });

// test('Ticket purchased multiple times before attending conference & OG seller onboards', async t => {
//     const { keypom, funder, bob, root, mintbase } = t.context.accounts;

//     let sellerKeys = await generateKeyPairs(1);
//     const basePassword = "MASTER_KEY"
//     let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
//         publicKeys: sellerKeys.publicKeys,
//         basePassword,
//         uses: [1]
//     });
//     await funder.call(keypom, 'create_drop', createDropArgs({
//         pubKeys: sellerKeys.publicKeys, 
//         passwords, 
//         root, 
//         actualContracts,
//         actualMethods,
//         actualAmounts,
//         attachedDeposit,
//         extraAllowance,
//         wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), 
//         {gas: '300000000000000'}
//     );
    
//     // Original seller is walletless and then sells it to buyer 1
//     const buyer1 = await root.createSubAccount('buyer1');
//     const buyer2 = await root.createSubAccount('buyer2');
//     await buyer1.call(mintbase, 'deposit_storage', {},{attachedDeposit: NEAR.parse("10").toString()});
//     await buyer2.call(mintbase, 'deposit_storage', {},{attachedDeposit: NEAR.parse("10").toString()});

//     let buyerKeys = await generateKeyPairs(1);
//     await sellNFT({
//         keypom,
//         mintbase,
//         seller: keypom,
//         sellerKeys,
//         buyer: buyer1,
//         buyerKeys,
//         t,
//         tokenId: `0:0`,
//     });
//     let originalSellerKeys = sellerKeys;
//     sellerKeys = buyerKeys;

//     /*
//         Walletless Bob with Key A sells it to buyer 1 (with key B)
//         Key A -> $NEAR
//         Key B -> Ticket (owned by Bob)

//         Buyer 1 with Key B sells ticket to buyer 2 (with key C)
//         Key B -> None
//         Key C -> Ticket

//         Buyer 2 with Key C sells ticket to buyer 1 (with key D)
//         Key C -> None
//         Key D -> Ticket

//         etc....
//     */

//     // Put the first key up for sale (owner does NOT have a wallet here)
//     for (let i = 0; i < 6; i++) {
//         let seller = i % 2 === 0 ? buyer1 : buyer2;
//         let buyer = i % 2 === 0 ? buyer2 : buyer1;
//         let buyerKeys = await generateKeyPairs(1);

//         // Every loop, buyer 1 and 2 switch between who is the seller and who is the buyer
//         await sellNFT({
//             keypom,
//             mintbase,
//             seller,
//             sellerKeys,
//             buyer,
//             buyerKeys,
//             t,
//             tokenId: `0:0`,
//         });

//         sellerKeys = buyerKeys;
//     }

//     // Seller now uses their key to create a new account
//     await keypom.setKey(originalSellerKeys.keys[0]);
//     await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `og-seller.${root.accountId}`, new_public_key: originalSellerKeys.publicKeys[0]}, {gas: WALLET_GAS});

//     // Now that the new account was created, the seller's original key should have been deleted from keypom contract
//     try {
//         await keypom.view('get_drop_information', {key: originalSellerKeys.publicKeys[0]});
//         t.fail('Should have thrown an error');
//     } catch {
//         t.pass();
//     }

//     // Check if seller's account was successfully created and exists
//     const sellerAccount = await root.getAccount(`og-seller.${root.accountId}`);
//     const doesExist = await sellerAccount.exists();
//     t.assert(doesExist === true);
// });

// test('NFT with Owner Transferred, approved, and revoked using Keypom', async t => {
//     const { keypom, funder, bob, root, mintbase } = t.context.accounts;

//     let originalOwnerKeys = await generateKeyPairs(1);
//     const basePassword = "MASTER_KEY"
//     let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
//         publicKeys: originalOwnerKeys.publicKeys,
//         basePassword,
//         uses: [1]
//     });
//     await funder.call(keypom, 'create_drop', createDropArgs({
//         pubKeys: originalOwnerKeys.publicKeys, 
//         nftOwners: [funder.accountId], 
//         passwords, 
//         root, 
//         actualContracts,
//         actualMethods,
//         actualAmounts,
//         attachedDeposit,
//         extraAllowance,
//         wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), 
//         {gas: '300000000000000'}
//     );
    
//     let initialAllowance = (await getKeyInformation(keypom, originalOwnerKeys.publicKeys[0])).allowance;

//     let newOwnerKeys = await generateKeyPairs(1);
//     await keypom.setKey(originalOwnerKeys.keys[0]);
//     await keypom.call(keypom, 'nft_transfer', {receiver_id: bob.accountId, memo: newOwnerKeys.publicKeys[0]});
    
//     let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, newOwnerKeys.publicKeys[0]);
//     console.log('keyInfo after transfer: ', keyInfo)
//     t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
//     t.is(keyInfo.owner_id, bob.accountId);
//     initialAllowance = keyInfo.allowance;

//     /// FIRST APPROVAL
//     await keypom.setKey(newOwnerKeys.keys[0]);
//     await keypom.call(keypom, 'nft_approve', {account_id: mintbase.accountId});

//     keyInfo = await getKeyInformation(keypom, newOwnerKeys.publicKeys[0]);
    
//     t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
//     t.assert(keyInfo.approved_account_ids[mintbase.accountId] === 0);
//     t.is(keyInfo.next_approval_id, 1);
//     initialAllowance = keyInfo.allowance;

//     /// REVOKE FIRST APPROVAL
//     await keypom.call(keypom, 'nft_revoke', {account_id: mintbase.accountId});
//     keyInfo = await getKeyInformation(keypom, newOwnerKeys.publicKeys[0]);
    
//     t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
//     t.assert(Object.keys(keyInfo.approved_account_ids).length === 0);
//     t.is(keyInfo.next_approval_id, 1);
//     initialAllowance = keyInfo.allowance;


//     /// SECOND APPROVAL
//     await keypom.call(keypom, 'nft_approve', {account_id: mintbase.accountId});

//     keyInfo = await getKeyInformation(keypom, newOwnerKeys.publicKeys[0]);
    
//     t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
//     t.assert(keyInfo.approved_account_ids[mintbase.accountId] === 1);
//     t.is(keyInfo.next_approval_id, 2);

//     initialAllowance = keyInfo.allowance;

//     await keypom.call(keypom, 'nft_revoke_all', {});
//     keyInfo = await getKeyInformation(keypom, newOwnerKeys.publicKeys[0]);
    
//     t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
//     t.assert(Object.keys(keyInfo.approved_account_ids).length === 0);
//     t.is(keyInfo.next_approval_id, 2);
// });

test('Costs', async t => {
    const { keypom, funder, bob, root, mintbase } = t.context.accounts;

    await funder.call(keypom, 'create_drop', createDropArgs({
        pubKeys: [], 
        nftOwners: [], 
        passwords: [], 
        root,
        actualContracts,
        actualMethods,
        actualAmounts,
        attachedDeposit: NEAR.parse("0.5").toString(),
        extraAllowance, 
        wasmDirectory: `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`}), 
        {gas: '300000000000000'}
    );

    let numKeys = 20;
    let originalOwnerKeys = await generateKeyPairs(numKeys);
    const basePassword = "MASTER_KEY"
    let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
        publicKeys: originalOwnerKeys.publicKeys,
        basePassword,
        uses: [1]
    });

    await funder.call(keypom, 'add_keys', {
        drop_id: '0', 
        public_keys: originalOwnerKeys.publicKeys, 
        key_owners: Array(numKeys).fill(funder.accountId),
        extra_key_allowance: extraAllowance,
        passwords_per_use: passwords
    }, {gas: '300000000000000'});
});