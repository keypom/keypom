import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist, assertNFTKeyData } from "../utils/general";
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
    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);

    const nftContract1 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    await nftContract1.call(nftContract1, 'new_default_meta', { owner_id: nftContract1});

    // const nftContract2 = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    // await nftContract2.call(nftContract2, 'new_default_meta', { owner_id: nftContract2});
    
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
    
    // Save state for test runs
    t.context.worker = worker;
    // t.context.accounts = { root, funder, keypomV3, ali };
    t.context.accounts = { root, funder, nftContract1, keypomV3, ali };

    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Withdraw multiple NFTs from the same asset both by specifying token IDs & using limit flag (both none & some).

// Ensure if panic, 
//      Everything in batch is reverted even correct transfers
//      Net state added is zero and no user balance is incremented
//      Tokens are correctly added back in the order they were removed
//      False is returned from function

// Ensure if no panic:
//      Tokens correctly transferred
//      Internal NFT data updated correctly
//      State cleared properly
//      Refund to user balance (claim logic + transfer + callback logic gas)
//      True is returned from function


test('Withdraw with TokenId - No Panic', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali} = t.context.accounts;
    
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    let numNFTs = 12
    const nftAsset1: ExtNFTData = {
        nft_contract_id: nftContract1.accountId
    }

    const asset_data_per_use = [
        {
            assets: [nftAsset1],
            uses: 1
        },
    ]

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
        },
    }) 

    // ******************* Adding Assets *******************
    let preSendBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let token_ids: string[] = []
    for(let i = 0; i < numNFTs; i++){
        let token_id = `NFT-TOKEN-NUMBER-${i+1}`

        await functionCall({
            signer: funder,
            receiver: nftContract1,
            methodName: 'nft_mint',
            args: {
                token_id,
                metadata: {
                    title: "my token",
                    description: 'Coming off a maiden victory at the Sahlen Six Hours of the Glen, the BMW Team RLL squad looks to repeat at Canadian Tire Motorsports following a disappointing qualifying. - Shot On: Nikon 55-200 f4-5.6',
                    media: 'https://ipfs.near.social/ipfs/bafybeig4hirpwvr2suakpwhikwfs4f2tjd5hky233k3fpzfeq6npz72fuy',
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
                token_id,
                msg: dropId
            },
            attachedDeposit: "1"
        })

        token_ids.push(token_id)
    }

    let postSendBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let balChange = formatNearAmount((BigInt(preSendBal) - BigInt(postSendBal)).toString(), 5);
    console.log(balChange)
    t.is(balChange > "0", true)

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract1.accountId,
            token_ids
        }],
    })

    let bal1: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    
    // Withdraw last 3 tokens by Token ID
    let withdrawResponse = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "withdraw_nft_tokens",
        args: {
            drop_id: dropId,
            nft_contract_id: nftContract1.accountId,
            token_ids: token_ids.slice(9, 12)

        }
    }) 
    t.is(withdrawResponse == "true", true)

    // Ensure token transfer succeeded
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract1.accountId,
            token_ids: token_ids.slice(0, 9)
        }],
    })

    let bal2: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    t.is(formatNearAmount((BigInt(bal2) - BigInt(bal1)).toString(), 5) > "0", true)

    console.log("Withdrawing by Limit")
    // Withdraw last 3 tokens by Limit
    withdrawResponse = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "withdraw_nft_tokens",
        args: {
            drop_id: dropId,
            nft_contract_id: nftContract1.accountId,
            limit: 3

        }
    }) 
    t.is(withdrawResponse == "true", true)

    // Ensure token transfer succeeded
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract1.accountId,
            token_ids: token_ids.slice(0, 6)
        }],
    })

    let bal3: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    t.is(formatNearAmount((BigInt(bal3) - BigInt(bal2)).toString(), 5) > "0", true)

    console.log("Withdrawing with nothing specified")
    // Withdraw last 3 tokens by Limit
    withdrawResponse = await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: "withdraw_nft_tokens",
        args: {
            drop_id: dropId,
            nft_contract_id: nftContract1.accountId,
        }
    }) 
    t.is(withdrawResponse == "true", true)

    // Ensure token transfer succeeded
    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract1.accountId,
            token_ids: []
        }],
    })

    let bal4: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    t.is(formatNearAmount((BigInt(bal4) - BigInt(bal3)).toString(), 5) > "0", true)
});

test('Withdraw with TokenId - Panic', async t => {
    const {funder, keypomV3, root, ftContract1, ftContract2,  nftContract1, ali} = t.context.accounts;

    const dropId = "my-drop-id";
    const numKeys = 2;
    let keyPairs = await generateKeyPairs(numKeys);

    // ******************* Creating Drop *******************
    let numNFTs = 4
    const nftAsset1: ExtNFTData = {
        nft_contract_id: nftContract1.accountId
    }

    const asset_data_per_use = [
        {
            assets: [nftAsset1],
            uses: 1
        },
    ]

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
        },
    }) 

    // ******************* Adding Assets (DO NOT EXIST, WILL PANIC ON WITHDRAW) *******************
    let preSendBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let token_ids: string[] = []
    for(let i = 0; i < numNFTs; i++){
        let token_id = `NFT-TOKEN-NUMBER-${i+1}`

        await functionCall({
            signer: nftContract1,
            receiver: keypomV3,
            methodName: 'nft_on_transfer',
            args: {
                token_id,
                sender_id: funder.accountId,
                msg: dropId
            },
        })

        token_ids.push(token_id)
    }

    let postSendBal: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    let balChange = formatNearAmount((BigInt(preSendBal) - BigInt(postSendBal)).toString(), 5);
    console.log(balChange)
    t.is(balChange > "0", true)

    await assertKeypomInternalAssets({
        keypom: keypomV3,
        dropId,
        expectedNftData: [{
            contract_id: nftContract1.accountId,
            token_ids
        }],
    })

    let initialBal = await keypomV3.balance();
    let bal1: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})

    console.log("Withdrawing Last 2 Tokens by Limit")
    // Withdraw last 2 tokens by Limit
    try{
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: "withdraw_nft_tokens",
            args: {
                drop_id: dropId,
                nft_contract_id: nftContract1.accountId,
                limit: 2
    
            }
        }) 
    }catch{
        // Ensure token transfer succeeded
        await assertKeypomInternalAssets({
            keypom: keypomV3,
            dropId,
            expectedNftData: [{
                contract_id: nftContract1.accountId,
                token_ids
            }],
        })
    }

    let bal2: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    t.is(formatNearAmount((BigInt(bal2) - BigInt(bal1)).toString(), 5) == "0", true) 

    console.log("Withdrawing with nothing specified")
    // Withdraw last 3 tokens by Limit
    try{
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: "withdraw_nft_tokens",
            args: {
                drop_id: dropId,
                nft_contract_id: nftContract1.accountId,
            }
        }) 
    }catch{
         // Ensure token transfer succeeded
        await assertKeypomInternalAssets({
            keypom: keypomV3,
            dropId,
            expectedNftData: [{
                contract_id: nftContract1.accountId,
                token_ids
            }],
        })
    }   
    
    let bal3: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    t.is(formatNearAmount((BigInt(bal3) - BigInt(bal2)).toString(), 5) == "0", true)

    console.log("Withdrawing by TokenId")
    try{
        let withdrawResponse = await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: "withdraw_nft_tokens",
            args: {
                drop_id: dropId,
                nft_contract_id: nftContract1.accountId,
                token_ids: token_ids.slice(1, 3)
    
            }
        }) 
        t.fail()
    }catch{
        // Will re-insert token 2 and 3 to back --> EXPECTED
        await assertKeypomInternalAssets({
            keypom: keypomV3,
            dropId,
            expectedNftData: [{
                contract_id: nftContract1.accountId,
                token_ids: [token_ids[0], token_ids[3], token_ids[1], token_ids[2]]
            }],
        })
    }

    let bal4: number = await keypomV3.view('get_user_balance', {account_id: funder.accountId})
    t.is(formatNearAmount((BigInt(bal4) - BigInt(bal3)).toString(), 5) == "0", true)

    let finallBal = await keypomV3.balance();
    t.deepEqual(finallBal.stateStaked, initialBal.stateStaked)
});

