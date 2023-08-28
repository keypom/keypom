import anyTest, { TestFn } from "ava";
import { ExecutionStatusBasic, KeyPair, NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, LARGE_GAS, WALLET_GAS, claimWithRequiredGas, functionCall, doesKeyExist, doesDropExist, displayBalances } from "../utils/general";
import { DropConfig } from "../utils/types";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    keypomInitialBalance: NEAR;
    keypomInitialStateStaked: NEAR;
}>;

test.beforeEach(async (t) => {
    // Comment this if you want to see console logs
    // console.log = function() {}

    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy all 3 contracts
    const keypomV3 = await root.devDeploy(`./out/keypom.wasm`);
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nested-fields-nft.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: 'test.near', owner_id: keypomV3, contract_metadata: CONTRACT_METADATA });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });

    // Test users
    const ali = await root.createSubAccount('ali');
    const funder = await root.createSubAccount('funder');
    const bob = await root.createSubAccount('bob');

    let keypomBalance = await keypomV3.balance();
    console.log('keypomV3 available INITIAL: ', keypomBalance.available.toString())
    console.log('keypomV3 staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypomV3 stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypomV3 total INITIAL: ', keypomBalance.total.toString())

    let nftBalance = await nftContract.balance();
    console.log('nftContract available INITIAL: ', nftBalance.available.toString())
    console.log('nftContract staked INITIAL: ', nftBalance.staked.toString())
    console.log('nftContract stateStaked INITIAL: ', nftBalance.stateStaked.toString())
    console.log('nftContract total INITIAL: ', nftBalance.total.toString())

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypomV3, nftContract, funder, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

const TERA_GAS = 1000000000000;

test('Top Level Fields', async t => {
    const { keypomV3, nftContract, funder, ali, bob, root } = t.context.accounts;
    let initialBal = await keypomV3.balance()

    let method1 = {
        receiver_id: nftContract.accountId,
        method_name: 'nft_mint',
        args: JSON.stringify({
            receiver_id: ali.accountId,
            token_id: '1',
            metadata: {}
        }),
        attached_deposit: NEAR.parse("1").toString(),
        attached_gas: (20 * TERA_GAS).toString(),
        keypom_args: {
            account_id_field: "account_id",
            key_id_field: "key_id",
            funder_id_field: "funder_id",
            drop_id_field: "drop_id",
        },
    }

    const fcAsset1 = [method1]
    // const fcAsset1 = null


    const dropId = "drop-id";
    let {keys, publicKeys} = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: [{
                assets: [fcAsset1],
                uses: 1
            }],
            key_data: [
                {
                    public_key: publicKeys[0]
                }
            ],
        },
        attachedDeposit: NEAR.parse("21").toString()
    })

    // This should pass and none of the user provided args should be used.
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId


    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].account_id, claimingAccount);
    t.is(aliTokens[0].funder_id, funder.accountId);
    t.is(aliTokens[0].key_id, "0");
    t.is(aliTokens[0].drop_id, dropId);

    t.is(await doesKeyExist(keypomV3, publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    // t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

test('Nested One Level Down', async t => {
    const { keypomV3, nftContract, funder, ali, bob, root } = t.context.accounts;
    let initialBal = await keypomV3.balance()

    const method1 = {
        receiver_id: nftContract.accountId,
        method_name: 'nft_mint',
        args: JSON.stringify({
            receiver_id: ali.accountId,
            token_id: '1',
            metadata: {}
        }),
        attached_deposit: NEAR.parse("1").toString(),
        attached_gas: (20 * TERA_GAS).toString(),
        keypom_args:{
            account_id_field: "metadata.account_id",
            key_id_field: "metadata.key_id",
            funder_id_field: "metadata.funder_id",
            drop_id_field: "metadata.drop_id",
        },
    }
    const fcAsset1 = [method1]


    const dropId = "drop-id";
    let {keys, publicKeys} = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: [{
                assets: [fcAsset1],
                uses: 1
            }],
            key_data: [
                {
                    public_key: publicKeys[0]
                }
            ],
        },
        attachedDeposit: NEAR.parse("21").toString()
    })

    // This should pass and none of the user provided args should be used.
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId



    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].metadata.drop_id, dropId);
    t.is(aliTokens[0].metadata.funder_id, funder.accountId);
    t.is(aliTokens[0].metadata.key_id, "0");
    t.is(aliTokens[0].metadata.account_id, claimingAccount);

    t.is(await doesKeyExist(keypomV3, publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    // t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});


test('Nested Two Levels Down', async t => {
    const { keypomV3, nftContract, funder, ali, bob, root } = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const method1 = {
        receiver_id: nftContract.accountId,
        method_name: 'nft_mint',
        args: JSON.stringify({
            receiver_id: ali.accountId,
            token_id: '1',
            metadata: {}
        }),
        attached_deposit: NEAR.parse("1").toString(),
        attached_gas: (20 * TERA_GAS).toString(),
        keypom_args:{
            account_id_field: "metadata.nested.account_id",
            key_id_field: "metadata.nested.key_id",
            funder_id_field: "metadata.nested.funder_id",
            drop_id_field: "metadata.nested.drop_id",
        },
    }
    const fcAsset1 = [method1]


    const dropId = "drop-id";
    let {keys, publicKeys} = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: [{
                assets: [fcAsset1],
                uses: 1
            }],
            key_data: [
                {
                    public_key: publicKeys[0]
                }
            ],
        },
        attachedDeposit: NEAR.parse("21").toString()
    })

    // This should pass and none of the user provided args should be used.
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId



    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].metadata.nested.account_id, claimingAccount);
    t.is(aliTokens[0].metadata.nested.funder_id, funder.accountId);
    t.is(aliTokens[0].metadata.nested.key_id, "0");
    t.is(aliTokens[0].metadata.nested.drop_id, dropId);

    t.is(await doesKeyExist(keypomV3, publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    // t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

test('Nested Fields that Dont Exist', async t => {
    const { keypomV3, nftContract, funder, ali, bob, root } = t.context.accounts;
    let initialBal = await keypomV3.balance()

    const method1 = {
        receiver_id: nftContract.accountId,
        method_name: 'nft_mint',
        args: JSON.stringify({
            receiver_id: ali.accountId,
            token_id: '1',
        }),
        attached_deposit: NEAR.parse("1").toString(),
        attached_gas: (20 * TERA_GAS).toString(),
        keypom_args:{
            account_id_field: "metadata.nested.account_id",
            key_id_field: "metadata.nested.key_id",
            funder_id_field: "metadata.nested.funder_id",
            drop_id_field: "metadata.nested.drop_id",
        },
    }
    const fcAsset1 = [method1]


    const dropId = "drop-id";
    let {keys, publicKeys} = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: [{
                assets: [fcAsset1],
                uses: 1
            }],
            key_data: [
                {
                    public_key: publicKeys[0]
                }
            ],
        },
        attachedDeposit: NEAR.parse("21").toString()
    })

    // This should pass and none of the user provided args should be used.
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId



    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].metadata.nested.account_id, claimingAccount);
    t.is(aliTokens[0].metadata.nested.funder_id, funder.accountId);
    t.is(aliTokens[0].metadata.nested.key_id, "0");
    t.is(aliTokens[0].metadata.nested.drop_id, dropId);

    t.is(await doesKeyExist(keypomV3, publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    // t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});

test('Nested Fields That Are Not an Object', async t => {
    const { keypomV3, nftContract, funder, ali, bob, root } = t.context.accounts;
    let initialBal = await keypomV3.balance();

    const method1 = {
        receiver_id: nftContract.accountId,
        method_name: 'nft_mint',
        args: JSON.stringify({
            receiver_id: ali.accountId,
            token_id: '1',
            metadata: {}
        }),
        attached_deposit: NEAR.parse("1").toString(),
        attached_gas: (20 * TERA_GAS).toString(),
        keypom_args:{
            account_id_field: "token_id.account_id",
            key_id_field: "token_id.key_id",
            funder_id_field: "token_id.funder_id",
            drop_id_field: "token_id.drop_id",
        },
    }
    const fcAsset1 = [method1]


    const dropId = "drop-id";
    let {keys, publicKeys} = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: [{
                assets: [fcAsset1],
                uses: 1
            }],
            key_data: [
                {
                    public_key: publicKeys[0]
                }
            ],
        },
        attachedDeposit: NEAR.parse("21").toString()
    })

    // This should pass and none of the user provided args should be used.
    let result: {response: string|undefined, actualReceiverId: string|undefined} = await claimWithRequiredGas({
        keypom: keypomV3,
        root,
        keyPair: keys[0],
        createAccount: true,
    })
    t.is(result.response, "true")
    let claimingAccount: string = result.actualReceiverId == undefined ? "" : result.actualReceiverId

    // Function call will fail
    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 0);

    t.is(await doesKeyExist(keypomV3, publicKeys[0]), false)
    t.is(await doesDropExist(keypomV3, dropId), false)

    let finalBal = await keypomV3.balance();
    displayBalances(initialBal, finalBal);
    // t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
});