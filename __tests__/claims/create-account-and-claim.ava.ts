import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, ExtNFTData, InternalNFTData } from "../utils/types";
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


    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);

    let nftAccounts: NearAccount[] = []

    const nftContract1 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    nftAccounts.push(nftContract1)
    

    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"} });

    for(let i = 0; i < nftAccounts.length; i++){
        await nftAccounts[i].call(nftAccounts[i], 'new_default_meta', { owner_id: nftAccounts[i]});
        await functionCall({
            signer: funder,
            receiver: nftAccounts[i],
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
    }


    const ftContract1 = await root.createSubAccount('ft_contract_1');
    // const ftContract2 = await root.createSubAccount('ft_contract_2');


    await ftContract1.deploy(`./__tests__/ext-wasm/ft.wasm`);
    // await ftContract2.deploy(`./__tests__/ext-wasm/ft.wasm`);

    
    await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });
    // await ftContract2.call(ftContract2, 'new_default_meta', { owner_id: ftContract2, total_supply: totalSupply.toString() });


    // Deposit storage
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    // // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})


    // // Send FTs
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})
    // // await functionCall({signer: ftContract2, receiver: ftContract2, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})

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
    // t.context.accounts = { root, funder, keypomV3, nftContract1, ali };
    t.context.accounts = { root, funder, keypomV3, 
        nftContract1,ftContract1,
        ali };

    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

 //EXPECTED CAAC FAILIURE BEHAVIOUR
 //Failed CAAC:
 //- no assets transferred
 //- Check if key is properly deleted in the case that it’s 0 remaining uses and check if it isn’t deleted in other case
 //- Make sure drop is deleted ONLY if it’s empty of keys and has no funded assets in it (i.e FTs & NFTs)
 //- If drop is deleted, only NEAR assets and FT registration cost should be refunded back to user balance


 //- account creation failed -> should refund ALL assets & key is decremented
 //- account creation succeeded but asset claims failed -> should refund assets that failed and do nothing for ones that weren’t

// account creation failed -> should refund ALL assets & key is decremented (NFT version)
test('Account Creation Fail in CAAC - drop still contains NFTs', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract1, nftContract2, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    const nftAsset1: ExtNFTData = {
        nft_contract_id: nftContract1.accountId
    }

    const asset_data = [
        {
            assets: [nftAsset1],
            uses: 1
        },
        {
            assets: [null],
            uses: 1
        }
    ]
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0]
            }]
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
            receiver: nftContract1,
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
            receiver: nftContract1,
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
            contract_id: nftContract1.accountId,
            token_ids: tokenIds
        },],
    })

    // 2 uses at the start
    let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 2)

    // First failed claim
    let result: {response: string|undefined, actualReceiverId: string | undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        receiverId: ali.accountId,
        shouldPanic: true
    })
    t.is(result.response, "false")

    // Key uses should have decremented
    keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 1)

    // Second failed claim
    result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        receiverId: ali.accountId,
        shouldPanic: true
    })
    t.is(result.response, "false")

    // Token should still be owned by Keypom
    let token: {token_id: string, owner_id: string} = await nftContract1.view('nft_token', {token_id: tokenIds[0]});
    console.log(`${token.token_id} is owned by ${token.owner_id}`)
    t.is(token.owner_id, keypomV3.accountId)

    // Key should be deleted but drop should still exist since asset is still there
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)

    // Drop should still have assets
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract1.accountId,
            token_ids: tokenIds
        },],
    })

});

// account creation failed -> should refund ALL assets & key is decremented (FT and NEAR asset version)
test('Account Creation Fail in CAAC - drop still contains FTs, refund NEAR from FT and NEAR assets', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nearAssetAmount: number = 1
    const ftRegistrationCost: number =  1

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse(ftRegistrationCost.toString()).toString(),
        ft_amount: "1"
    }

    const nearAsset1 = {
        yoctonear: NEAR.parse(nearAssetAmount.toString()).toString()
    }

    const asset_data = [
        {
            assets: [ftAsset1, nearAsset1],
            uses: 1
        },
        {
            assets: [null],
            uses: 1
        }
    ]
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0]
            }]
        },
        attachedDeposit: NEAR.parse("1").toString(),
    })

    await sendFTs(funder, "1", keypomV3, ftContract1, dropId)
    

    // Assert Assets
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '1',
        }]
    })

    // 2 uses at the start
    let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 2)

    // starting claim NEAR and FT balance
    let preClaimFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

    // First failed claim
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        receiverId: ali.accountId,
        shouldPanic: true
    })
    t.is(result.response, "false")

    // Key uses should have decremented
    keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 1)

    // Ensure no asset transfer
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '1',
        }]
    })

    // NEAR asset and FT registration cost should have been refunded here
    let refundedFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let refund = formatNearAmount((BigInt(refundedFunderBal) - BigInt(preClaimFunderBal)).toString(), 5);
    const EXPECTED_REFUND_NEAR: number = ftRegistrationCost + nearAssetAmount;
    t.is(refund.toString() >= EXPECTED_REFUND_NEAR.toString(), true)

    // Second failed claim
    result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        receiverId: ali.accountId,
        shouldPanic: true
    })
    t.is(result.response, "false")

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), true)

    // Drop should still have assets
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '1',
        }]
    })

});

// account creation succeeded but asset claims failed -> should refund assets that failed and do nothing for ones that weren’t
test('Asset Claim Failure in CAAC', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2, nftContract, nftContract2, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const nearAssetAmount: number = 2
    const ft1RegistrationCost: number =  5
    const ft2RegistrationCost: number =  1

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    const ftAsset1: ExtFTData = {
        ft_contract_id: ftContract1.accountId,
        registration_cost: NEAR.parse(ft1RegistrationCost.toString()).toString(),
        ft_amount: "1"
    }

    const ftAsset2: ExtFTData = {
        ft_contract_id: "foo-contract",
        registration_cost: NEAR.parse(ft2RegistrationCost.toString()).toString(),
        ft_amount: '0'
    }

    const nearAsset1 = {
        yoctonear: NEAR.parse(nearAssetAmount.toString()).toString()
    }

    const asset_data = [
        {
            assets: [ftAsset1, nearAsset1, ftAsset2],
            uses: 1
        },
        {
            assets: [null],
            uses: 1
        }
    ]
    
    
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: [{
                public_key: keyPairs.publicKeys[0]
            }]
        },
        attachedDeposit: NEAR.parse("10").toString(),
    })

    await sendFTs(funder, "1", keypomV3, ftContract1, dropId)
    

    // Assert Assets
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '1',
        },
        {
            contract_id: "foo-contract",
            balance_avail: '0'
        }
    ]
    })

    // 2 uses at the start
    let keyInfo: {uses_remaining: number} = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 2)

    // starting claim balance
    let preClaimFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

    // First claim - will fail due to fake FT contract
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        shouldPanic: true
    })
    t.is(result.response, "false")

    // Key uses should have decremented
    keyInfo = await keypomV3.view('get_key_information', {key: keyPairs.publicKeys[0]});
    t.is(keyInfo.uses_remaining, 1)

    // Ensure proper FT transfer
    let userPostClaimFTBal: number = await ftContract1.view("ft_balance_of", {account_id: result.actualReceiverId});
    console.log(`${result.actualReceiverId} FT bal on ${ftAsset1.ft_contract_id}: ${userPostClaimFTBal}`)
    // CAAC, starting FT bal was 0
    t.is(userPostClaimFTBal == 1, true)

    // Ensure proper NEAR asset transfer
    let userBal: {available: NEAR} = await root.getAccount(result.actualReceiverId == undefined ? "" : result.actualReceiverId).balance()
    console.log('userBal: ', userBal.available.toString());
    t.is(userBal.available.gte(NEAR.parse(nearAssetAmount.toString())), true)
    
    //ensure refund from failed ft claim is sent back, but NOT near asset OR successful FT claim
    let refundedFunderBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let refund = formatNearAmount((BigInt(refundedFunderBal) - BigInt(preClaimFunderBal)).toString(), 5);
    console.log(`Refunded Amount: ${refund}`)
    // ft2 failed, therefore only ft2 deposit should be refuned
    const EXPECTED_REFUND_NEAR: number = ft2RegistrationCost;
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedFtData: [{
            contract_id: ftContract1.accountId,
            balance_avail: '0',
        },
        {
            contract_id: "foo-contract",
            balance_avail: '0'
        }
    ]
    })
    // Ensure NEAR asset was not refunded
    t.is(
        // Failed FT2 asset registration cost should be refunded
        refund.toString() >= EXPECTED_REFUND_NEAR.toString() 
        // Ensure NEAR asset was not refunded
        && refund.toString() < nearAssetAmount.toString()
        // Ensure successful FT1 asset registration cost is not refunded
        && refund.toString() < ft1RegistrationCost.toString(), true)
    

    // Second claim, check refund of next registration cost
    // First claim - will fail due to fake FT contract
    result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")

    // Assets all gone
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
});
