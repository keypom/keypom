import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist, assertNFTKeyData, assertProperStorage } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNFTData, InternalNFTData, InternalFTData, ExtNearData, TokenMetadata } from "../utils/types";
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

// Verify that Metadata + Royalty checker works
test('Metadata and Royalties Helper Test', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* SETUP *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data = [{
        assets: [nearAsset1],
        uses: 1
    }];

    let royalties: Record<string, number>={
    }
    // 100000 is 100% 
    royalties[keypomV3.accountId] = 6500
    royalties[funder.accountId] = 3000
    royalties[bob.accountId] = 500

    let metadata: TokenMetadata = {
        title: "my token",
        description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
        media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
        media_hash: undefined,
        copies: undefined,
        issued_at: undefined,
        expires_at: undefined,
        starts_at: undefined,
        updated_at: undefined,
        extra: undefined,
        reference: undefined,
        reference_hash: undefined
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
                key_owner: funder
            }],
            drop_config: {
                nft_keys_config: {
                    token_metadata: metadata,
                    royalties
                }
            }
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})

    // **************** RECORD KEYS DIFFERENT ****************
    let diffKeysRoyalties: Record<string, number>={
    }
    // 100000 is 100% 
    diffKeysRoyalties[keypomV3.accountId] = 6500
    diffKeysRoyalties[ali.accountId] = 3000
    diffKeysRoyalties[bob.accountId] = 500
    
    let royalties_and_metadata_same: {royaltySame: boolean, metadataSame: boolean} = await assertNFTKeyData({
        keypom: keypomV3,
        tokenId: found_key_info.token_id,
        expectedRoyalties: diffKeysRoyalties,
        expectedMetadata: metadata
    })
    t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), false)
    
    // **************** RECORD VALUES DIFFERENT ****************
    let diffValuesRoyalties: Record<string, number>={
    }
    // 100000 is 100% 
    diffValuesRoyalties[keypomV3.accountId] = 3000
    diffValuesRoyalties[funder.accountId] = 6000
    diffValuesRoyalties[bob.accountId] = 1000

    royalties_and_metadata_same = await assertNFTKeyData({
        keypom: keypomV3,
        tokenId: found_key_info.token_id,
        expectedRoyalties: diffValuesRoyalties,
        expectedMetadata: metadata
    })
    t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), false)

    // **************** RECORD LENGTHS DIFFERENT ****************
    let shortRoyalties: Record<string, number>={
    }
    // 100000 is 100% 
    shortRoyalties[keypomV3.accountId] = 6500
    shortRoyalties[funder.accountId] = 3000

    royalties_and_metadata_same = await assertNFTKeyData({
        keypom: keypomV3,
        tokenId: found_key_info.token_id,
        expectedRoyalties: shortRoyalties,
        expectedMetadata: metadata
    })
    t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), false)


    // **************** RECORD LENGTHS DIFFERENT ****************
    let badMetadata: TokenMetadata = {
        title: "your token",
        description: 'abc',
        media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
        media_hash: undefined,
        copies: undefined,
        issued_at: undefined,
        expires_at: undefined,
        starts_at: undefined,
        updated_at: undefined,
        extra: undefined,
        reference: undefined,
        reference_hash: undefined
    }

    royalties_and_metadata_same = await assertNFTKeyData({
        keypom: keypomV3,
        tokenId: found_key_info.token_id,
        expectedRoyalties: royalties,
        expectedMetadata: badMetadata
    })
    t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), false)
    

    // **************** WORKING CASE ****************
    royalties_and_metadata_same = await assertNFTKeyData({
        keypom: keypomV3,
        tokenId: found_key_info.token_id,
        expectedRoyalties: royalties,
        expectedMetadata: metadata
    })
    t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), true)
});

test('Storage Helper Test', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* SETUP *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data = [{
        assets: [nearAsset1],
        uses: 1
    }];

    let royalties: Record<string, number>={
    }
    // 100000 is 100% 
    royalties[keypomV3.accountId] = 6500
    royalties[funder.accountId] = 3000
    royalties[bob.accountId] = 500

    let metadata: TokenMetadata = {
        title: "my token",
        description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
        media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
        media_hash: undefined,
        copies: undefined,
        issued_at: undefined,
        expires_at: undefined,
        starts_at: undefined,
        updated_at: undefined,
        extra: undefined,
        reference: undefined,
        reference_hash: undefined
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
                key_owner: funder
            }],
            drop_config: {
                nft_keys_config: {
                    token_metadata: metadata,
                    royalties
                }
            }
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})

    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: funder
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)
});

test('Ownerless Keys', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = [{
        assets: [nearAsset1],
        uses: 1
    }];

    let royalties: Record<string, number>={
    }
    // 100000 is 100% 
    royalties[keypomV3.accountId] = 6500
    royalties[funder.accountId] = 3000
    royalties[bob.accountId] = 500

    let metadata: TokenMetadata = {
        title: "my token",
        description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
        media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
        media_hash: undefined,
        copies: undefined,
        issued_at: undefined,
        expires_at: undefined,
        starts_at: undefined,
        updated_at: undefined,
        extra: undefined,
        reference: undefined,
        reference_hash: undefined
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: asset_data_per_use,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
            }],
            drop_config: {
                nft_keys_config: {
                    token_metadata: metadata,
                    royalties
                }
            }
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    t.is(found_key_info.owner_id == keypomV3.accountId, true)

    let royalties_and_metadata_same: {royaltySame: boolean, metadataSame: boolean} = await assertNFTKeyData({
        keypom: keypomV3,
        tokenId: found_key_info.token_id,
        expectedRoyalties: royalties,
        expectedMetadata: metadata
    })
    t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), true)

    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: keypomV3
    })
    console.log(storageBools.token_id_by_pk_check)
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)
});

test('Owned Keys', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data_per_use = [{
        assets: [nearAsset1],
        uses: 1
    }];

    let royalties: Record<string, number>={
    }
    // 100000 is 100% 
    royalties[keypomV3.accountId] = 6500
    royalties[funder.accountId] = 3000
    royalties[bob.accountId] = 500

    let metadata: TokenMetadata = {
        title: "my token",
        description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
        media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
        media_hash: undefined,
        copies: undefined,
        issued_at: undefined,
        expires_at: undefined,
        starts_at: undefined,
        updated_at: undefined,
        extra: undefined,
        reference: undefined,
        reference_hash: undefined
    }
    
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
            drop_config: {
                nft_keys_config: {
                    token_metadata: metadata,
                    royalties
                }
            }
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    t.is(found_key_info.owner_id == funder.accountId, true)

    let royalties_and_metadata_same: {royaltySame: boolean, metadataSame: boolean} = await assertNFTKeyData({
        keypom: keypomV3,
        tokenId: found_key_info.token_id,
        expectedRoyalties: royalties,
        expectedMetadata: metadata
    })
    t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), true)

    let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
        keypom: keypomV3,
        expectedTokenId: found_key_info.token_id,
        keyPair: keyPairs.keys[0],
        expectedOwner: funder
    })
    t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)
});

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

    const asset_data = [{
        assets: [nearAsset1],
        uses: 1
    }];

    let royalties: Record<string, number>={
    }
    // 100000 is 100% 
    royalties[keypomV3.accountId] = 6500
    royalties[funder.accountId] = 3000
    royalties[bob.accountId] = 500

    let metadata: TokenMetadata = {
        title: "my token",
        description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
        media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
        media_hash: undefined,
        copies: undefined,
        issued_at: undefined,
        expires_at: undefined,
        starts_at: undefined,
        updated_at: undefined,
        extra: undefined,
        reference: undefined,
        reference_hash: undefined
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
                key_owner: funder
            }],
            drop_config: {
                nft_keys_config: {
                    token_metadata: metadata,
                    royalties
                }
            }
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    t.is(found_key_info.owner_id == funder.accountId, true)

    // **************** APPROVE AND REVOKE TESTING ****************
    try{
        // Add and Revoke, then try Transferring - SHOULD FAIL
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: "nft_approve",
            args:{
                token_id: found_key_info.token_id,
                account_id: ali.accountId
            }
        })
    
        let found_nft_info: {owner_id: string, approved_account_ids: Record<string, string>} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
        t.is(found_nft_info.approved_account_ids.hasOwnProperty(ali.accountId), true);
    
        // Remove Ali from list of 
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: "nft_revoke",
            args: {
                token_id: found_key_info.token_id,
                account_id: ali.accountId
            }
        })
    
        found_nft_info = await keypomV3.view("nft_token", {token_id: found_key_info.token_id});
        t.is(found_nft_info.approved_account_ids.hasOwnProperty(ali.accountId), false);
        
        // This should throw
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
        // If transfer does not throw error, fail the test.
        t.fail()
    }catch(e){
        // Ensure Revoke from Try block has worked
        let found_nft_info: {owner_id: string, approved_account_ids: Record<string, string>} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
        t.is(found_nft_info.approved_account_ids.hasOwnProperty(ali.accountId), false);

        // If worked, re-approve
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: "nft_approve",
            args:{
                token_id: found_key_info.token_id,
                account_id: ali.accountId
            }
        })
    
        found_nft_info = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
        t.is(found_nft_info.approved_account_ids.hasOwnProperty(ali.accountId), true);
    
    
        // **************** TRANSFER ****************
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

         // Ensure new owner + approved accounts list has reset
         let final_nft_token_info: {owner_id: string, royalty: Record<string, number>, metadata: TokenMetadata, approved_account_ids: Record<string, string>} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
         t.is(final_nft_token_info.owner_id == bob.accountId, true)
         t.is(final_nft_token_info.approved_account_ids.hasOwnProperty(ali.accountId), false)
 
         // **************** ENSURE ROYALTIES AND METADATA HAVE NOT CHANGED ****************
         let royalties_and_metadata_same: {royaltySame: boolean, metadataSame: boolean} = await assertNFTKeyData({
             keypom: keypomV3,
             tokenId: found_key_info.token_id,
             expectedRoyalties: royalties,
             expectedMetadata: metadata
         })
         t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), true)

         let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
            keypom: keypomV3,
            expectedTokenId: found_key_info.token_id,
            keyPair: newKeyPair.keys[0],
            expectedOwner: bob
        })
        t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)
    }
});

test('Owned key transfer, approval, revoking - with key', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali, bob} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* SETUP *******************
    const nearAsset1: ExtNearData = {
        yoctonear: NEAR.parse("1").toString()
    }

    const asset_data = [{
        assets: [nearAsset1],
        uses: 1
    }];

    let royalties: Record<string, number>={
    }
    // 100000 is 100% 
    royalties[keypomV3.accountId] = 6500
    royalties[funder.accountId] = 3000
    royalties[bob.accountId] = 500

    let metadata: TokenMetadata = {
        title: "my token",
        description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
        media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
        media_hash: undefined,
        copies: undefined,
        issued_at: undefined,
        expires_at: undefined,
        starts_at: undefined,
        updated_at: undefined,
        extra: undefined,
        reference: undefined,
        reference_hash: undefined
    }
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0],
                key_owner: funder
            }],
            drop_config: {
                nft_keys_config: {
                    token_metadata: metadata,
                    royalties
                }
            }
        },
    }) 

    let found_key_info: {owner_id: string, token_id: string} = await keypomV3.view("get_key_information", {key: keyPairs.publicKeys[0]})
    t.is(found_key_info.owner_id == funder.accountId, true)

    // **************** APPROVE AND REVOKE TESTING ****************
    try{
        // Add and Revoke, then try Transferring - SHOULD FAIL
        await keypomV3.setKey(keyPairs.keys[0]);
        await keypomV3.call(keypomV3, 'nft_approve', {token_id: found_key_info.token_id, account_id: ali.accountId});
        let found_nft_info: {owner_id: string, approved_account_ids: Record<string, string>} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
        t.is(found_nft_info.approved_account_ids.hasOwnProperty(ali.accountId), true);

        await keypomV3.call(keypomV3, 'nft_revoke', {token_id: found_key_info.token_id, account_id: ali.accountId});
        found_nft_info = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
        t.is(found_nft_info.approved_account_ids.hasOwnProperty(ali.accountId), false);

        // This should throw
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
        // If transfer does not throw error, fail the test.
        t.fail()
    }catch(e){
        // Ensure Revoke from Try block has worked
        let found_nft_info: {owner_id: string, approved_account_ids: Record<string, string>, royalty: Record<string, number>} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
        t.is(found_nft_info.approved_account_ids.hasOwnProperty(ali.accountId), false);

        // If worked, re-approve
        await keypomV3.call(keypomV3, 'nft_approve', {token_id: found_key_info.token_id, account_id: ali.accountId});
        found_nft_info = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
        t.is(found_nft_info.approved_account_ids.hasOwnProperty(ali.accountId), true);
    
    
        // **************** TRANSFER ****************
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

        // Ensure owner has changed and approved accounts list has reset
        let final_nft_token_info: {owner_id: string, royalty: Record<string, number>, metadata: TokenMetadata, approved_account_ids: Record<string, string>} = await keypomV3.view("nft_token", {token_id: found_key_info.token_id})
        t.is(final_nft_token_info.owner_id == bob.accountId, true)
        t.is(final_nft_token_info.approved_account_ids.hasOwnProperty(ali.accountId), false)

        // **************** ENSURE ROYALTIES AND METADATA HAVE NOT CHANGED ****************
        let royalties_and_metadata_same: {royaltySame: boolean, metadataSame: boolean} = await assertNFTKeyData({
            keypom: keypomV3,
            tokenId: found_key_info.token_id,
            expectedRoyalties: royalties,
            expectedMetadata: metadata
        })
        t.is((royalties_and_metadata_same.royaltySame && royalties_and_metadata_same.metadataSame), true)

        let storageBools: {tokens_per_owner_check: boolean, token_id_by_pk_check: boolean} = await assertProperStorage({
            keypom: keypomV3,
            expectedTokenId: found_key_info.token_id,
            keyPair: newKeyPair.keys[0],
            expectedOwner: bob
        })
        t.is(storageBools.tokens_per_owner_check && storageBools.token_id_by_pk_check, true)
    }
});





