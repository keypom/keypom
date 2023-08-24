import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker, toYocto } from "near-workspaces";
import { CONTRACT_METADATA, LARGE_GAS, assertKeypomInternalAssets, displayBalances, claimWithRequiredGas, functionCall, generateKeyPairs, initKeypomConnection, doesKeyExist, doesDropExist } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "../utils/ft-utils";
import { BN } from "bn.js";
import { ExtDrop, ExtFTData, InternalNFTData } from "../utils/types";
import { formatNearAmount } from "near-api-js/lib/utils/format";
import { generatePasswordsForClaim, generatePasswordsForKey } from "../utils/pwUtils";

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
    
    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, keypomV3, ali };
    // ftContract1, ftContract2
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// Normal Password Test
test('Password Claim', async t => {
    const {funder, keypomV3, root, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";
    let numKeys = 1;
    let useWithPw = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    let basePassword = 'mypassword1';
    let key_data: {public_key: string, password_by_use?: Record<number, string>}[] = [];
    for (var pk of keyPairs.publicKeys) {
        let password_by_use = generatePasswordsForKey(pk, [useWithPw], basePassword);
        
        key_data.push({
            public_key: pk,
            password_by_use
        })
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
    })

    t.is(await doesDropExist(keypomV3, dropId), true)

    // No PW, should fail
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        shouldPanic: true
    })

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), true)

    // Claim with PW
    result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        password: generatePasswordsForClaim(keyPairs.publicKeys[0], 1, basePassword)
    })
    t.is(result.response == "true", true)

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
});

test('Adding Keys with PW', async t => {
    const {funder, keypomV3, root, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";

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
            key_data: []
        },
    })

    t.is(await doesDropExist(keypomV3, dropId), true)

    let numKeys = 1;
    let useWithPw = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    let basePassword = 'mypassword1';
    let key_data: {public_key: string, password_by_use?: Record<number, string>}[] = [];
    for (var pk of keyPairs.publicKeys) {
        let password_by_use = generatePasswordsForKey(pk, [useWithPw], basePassword);
        
        key_data.push({
            public_key: pk,
            password_by_use
        })
    }

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data
        },
        attachedDeposit: NEAR.parse("10").toString(),
    })

    // No PW, should fail
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        shouldPanic: true
    })

    t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[0]), true)

    // Claim with PW
    result = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keyPairs.keys[0],
        createAccount: true,
        password: generatePasswordsForClaim(keyPairs.publicKeys[0], 1, basePassword)
    })
    t.is(result.response == "true", true)

    // Key should be deleted but drop should still exist
    t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)
});

test('Adding a Bunch of Keys with PW', async t => {
    const {funder, keypomV3, root, ali} = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const dropId = "my-drop-id";

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
            key_data: []
        },
    })

    t.is(await doesDropExist(keypomV3, dropId), true)

    let numKeys = 100;
    let useWithPw = 1;
    let keyPairs = await generateKeyPairs(numKeys);

    let basePassword = 'mypassword1';
    let key_data: {public_key: string, password_by_use?: Record<number, string>}[] = [];
    for (var pk of keyPairs.publicKeys) {
        let password_by_use = generatePasswordsForKey(pk, [useWithPw], basePassword);
        
        key_data.push({
            public_key: pk,
            password_by_use
        })
    }

    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data
        },
        attachedDeposit: NEAR.parse("10").toString(),
    })

    for(let i = 0; i < numKeys; i++){
        // No PW, should fail
        let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
            keypom: keypomV3,
            root,
            keyPair: keyPairs.keys[i],
            createAccount: true,
            shouldPanic: true
        })

        t.is(await doesKeyExist(keypomV3, keyPairs.publicKeys[i]), true)

        // Claim with PW
        result = await claimWithRequiredGas({
            keypom: keypomV3,
            root,
            keyPair: keyPairs.keys[i],
            createAccount: true,
            password: generatePasswordsForClaim(keyPairs.publicKeys[i], 1, basePassword)
        })
        t.is(result.response == "true", true)

        // Key should be deleted but drop should still exist
        t.is(await doesKeyExist(keypomV3,keyPairs.publicKeys[i]), false)
    }
    
    t.is(await doesDropExist(keypomV3, dropId), false)
});