import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, displayBalances, functionCall, generateKeyPairs, initKeypomConnection, claimWithRequiredGas, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtNFTData, InternalNFTData } from "../utils/types";
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
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });

    const ftContract1 = await root.createSubAccount('ft_contract_1');

    await ftContract1.deploy(`./__tests__/ext-wasm/ft.wasm`);
    
    await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });
    
    // Deposit storage
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})

    // Send FTs
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})

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
    t.context.accounts = { root, funder, keypomV3, nftContract, ftContract1 };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Not enough balance / deposit
test('Insufficient Balance + Deposit', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "drop-id";
    let numKeys = 90
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    const nearAsset1 = {
        yoctonear: NEAR.parse("10").toString()
    }
    const asset_data = [{
        assets: [nearAsset1],
        uses: 1
    }]

    // 90*10NEAR per use; balance + deposit must be more than 900 NEAR. This will panic
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data
        },
        attachedDeposit: "0",
        shouldPanic: true
    })


    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

// Add keys insufficient balance
test('Add Keys - Insufficient Balance + Deposit', async t => {
    const {funder, keypomV3, root} = t.context.accounts;

    const dropId = "drop-id";
    let numKeys = 90
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    const nearAsset1 = {
        yoctonear: NEAR.parse("10").toString()
    }
    const asset_data = [{
        assets: [nearAsset1],
        uses: 1
    }]

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: []
        },
        attachedDeposit: "0",
    })

    t.is(await doesDropExist(keypomV3, dropId), true)

    // State should not change when trying to add these keys, use balance to assert this
    let initialBal = await keypomV3.balance();

    // 90*10NEAR per use; balance + deposit must be more than 900 NEAR. This will panic
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data
        },
        shouldPanic: true
    })



    let keysForDrop = await keypomV3.view('get_key_supply_for_drop', {drop_id: dropId});
    console.log('keysForDrop: ', keysForDrop)
    t.is(keysForDrop, 0)


    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

// Conflicting public keys
test('Conflicting Keys', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "drop-id";
    let numKeys = 5
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }
    key_data.push({public_key: keyPairs.publicKeys[0]})

    const asset_data = [{
        assets: [null],
        uses: 1
    }]

    // First 5 are good and the last is repeated. No keys should be added
    // And the contract should panic
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data
        },
        attachedDeposit: NEAR.parse("10").toString(),
        shouldPanic: true
    })

    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

// Conflicting drop IDs
test('Conflicting DropIDs', async t => {
    const {funder, keypomV3, root} = t.context.accounts;

    const dropId = "drop-id";
    let numKeys = 5
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    const asset_data = [{
        assets: [null],
        uses: 1
    }]

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data
        },
        attachedDeposit: "0",
        shouldPanic: false
    })
    t.is(await doesDropExist(keypomV3, dropId), true)

    let initialBadBal = await keypomV3.balance();

    // Creating second drop with identical drop ID
    let keyPairs2 = await generateKeyPairs(numKeys);
    let key_data2: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data2.push({public_key: keyPairs2.publicKeys[i]})
    }

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: key_data2
        },
        attachedDeposit: "0",
        shouldPanic: true
    })

    // second failed drop should not change state
    let finalBal = await keypomV3.balance();
    displayBalances(initialBadBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBadBal.stateStaked);
});

// Really really really long drop ID
test('Really long DropIDs', async t => {
    const {funder, keypomV3, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    // Massive Drop ID, this far exceeds the 2048Byte limit and will panic
    const dropId = "Tying shoelaces is a fundamental skill that most people learn at an early age, and it is an important step towards becoming self-sufficient. Although it may seem like a simple task, mastering the art of tying shoes can ensure a secure and comfortable fit throughout the day. Here's a step-by-step guide to help you tie your shoes with ease and confidence. Start by holding one shoelace in each hand, making sure the lengths are equal. Cross the laces over each other, creating an X shape. Take the lace on the right side and pass it over the lace on the left side, threading it through the space between the two laces. Now, pull both ends of the laces firmly to tighten the knot, ensuring the knot is positioned near the base of the shoelaces. Congratulations, you've successfully tied the first part! Moving on, create two loops with the laces, holding one loop in each hand. Cross the loops over each other, forming another X shape. Take the loop in your right hand and pass it over the loop in your left hand, threading it through the space between the loops. Hold the loops firmly and pull them outward to tighten the knot, making sure the loops are of equal size. This is the crucial moment when your shoes start taking shape. To complete the process, grab the loops and give them a slight tug to secure the knot firmly. Optionally, you can tuck the loops under the shoelace knot for a cleaner look and to prevent tripping. Now, you're ready to confidently walk, run, or engage in any activity without worrying about loose shoelaces. Congratulations, you've mastered the basic bunny ears method! However, if you prefer an alternative method, you can try the Ian knot method, which is faster but may take some time to learn. Here's a brief overview of the Ian knot: Start by crossing the laces over each other, just like in the previous method. Take the lace on the right side and pass it under the lace on the left side. Next, bring the end of the right lace through the space between the laces, creating a loop aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa!";
    let numKeys = 5
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    const asset_data = [{
        assets: [null],
        uses: 1
    }]

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data
        },
        attachedDeposit: "0",
        shouldPanic: true
    })
    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

// Too many assets in a use leading to gas problems (try to force a panic as late down the road as possible i.e first 15 uses are fine but last one is not).
test('Asset Overload in Late Use', async t => {
    const {funder, keypomV3, nftContract, root} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "drop-id";
    let numKeys = 1
    let keyPairs = await generateKeyPairs(numKeys);
    let key_data: {public_key: string}[] = []
    for(let i = 0; i < numKeys; i++){
        key_data.push({public_key: keyPairs.publicKeys[i]})
    }

    const nftAsset1: ExtNFTData = {
        nft_contract_id: nftContract.accountId
    }
    
    const MAX_NUM_NFTS = 18
    const asset_data = [
        {
        // 18 NFT assets - max!
            assets: Array(MAX_NUM_NFTS).fill(nftAsset1),
            uses: 15
        },
        {
            // 19 should fail
                assets: Array(MAX_NUM_NFTS + 1).fill(nftAsset1),
                uses: 15
            },
    ]

    // First 5 are good and the last is repeated. No keys should be added
    // And the contract should panic
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data
        },
        attachedDeposit: NEAR.parse("10").toString(),
        shouldPanic: true
    })

    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});