import anyTest, { TestFn } from "ava";
import { ExecutionStatusBasic, NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, LARGE_GAS, WALLET_GAS, displayFailureLog } from "../utils/general";
import { DropConfig, FCData } from "../utils/types";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    keypomInitialBalance: NEAR;
    keypomInitialStateStaked: NEAR;
}>;

test.beforeEach(async (t) => {
    // Comment this if you want to see console logs
    //console.log = function() {}

    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy all 3 contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nested-fields-nft.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });

    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    let nftBalance = await nftContract.balance();
    console.log('nftContract available INITIAL: ', nftBalance.available.toString())
    console.log('nftContract staked INITIAL: ', nftBalance.staked.toString())
    console.log('nftContract stateStaked INITIAL: ', nftBalance.stateStaked.toString())
    console.log('nftContract total INITIAL: ', nftBalance.total.toString())

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const bob = await root.createSubAccount('bob');

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, nftContract, owner, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Top Level Fields', async t => {
    const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

    const fcData: FCData = {
        methods: [
            [
                {
                    receiver_id: nftContract.accountId,
                    method_name: 'nft_mint',
                    args: JSON.stringify({
                        receiver_id: ali.accountId,
                        token_id: '1',
                        metadata: {}
                    }),
                    account_id_field: "account_id",
                    key_id_field: "key_id",
                    funder_id_field: "funder_id",
                    drop_id_field: "drop_id",
                    attached_deposit: NEAR.parse("1").toString(),
                }
            ]
        ]
    }

    const config: DropConfig = { 
        uses_per_key: 1
    }

    let {keys, publicKeys} = await generateKeyPairs(1);
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
    await keypom.setKey(keys[0]);

    // This should pass and none of the user provided args should be used.
    const res = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
    displayFailureLog(res);

    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].account_id, bob.accountId);
    t.is(aliTokens[0].funder_id, ali.accountId);
    t.is(aliTokens[0].key_id, "0");
    t.is(aliTokens[0].drop_id, "0");
});

test('Nested One Level Down', async t => {
    const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

    const fcData: FCData = {
        methods: [
            [
                {
                    receiver_id: nftContract.accountId,
                    method_name: 'nft_mint',
                    args: JSON.stringify({
                        receiver_id: ali.accountId,
                        token_id: '1',
                        metadata: {}
                    }),
                    account_id_field: "metadata.account_id",
                    key_id_field: "metadata.key_id",
                    funder_id_field: "metadata.funder_id",
                    drop_id_field: "metadata.drop_id",
                    attached_deposit: NEAR.parse("1").toString(),
                }
            ]
        ]
    }

    const config: DropConfig = { 
        uses_per_key: 1
    }

    let {keys, publicKeys} = await generateKeyPairs(1);
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
    await keypom.setKey(keys[0]);

    // This should pass and none of the user provided args should be used.
    const res = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
    displayFailureLog(res);

    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].metadata.account_id, bob.accountId);
    t.is(aliTokens[0].metadata.funder_id, ali.accountId);
    t.is(aliTokens[0].metadata.key_id, "0");
    t.is(aliTokens[0].metadata.drop_id, "0");
});

test('Nested Two Levels Down', async t => {
    const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

    const fcData: FCData = {
        methods: [
            [
                {
                    receiver_id: nftContract.accountId,
                    method_name: 'nft_mint',
                    args: JSON.stringify({
                        receiver_id: ali.accountId,
                        token_id: '1',
                        metadata: {
                            nested: {}
                        }
                    }),
                    account_id_field: "metadata.nested.account_id",
                    key_id_field: "metadata.nested.key_id",
                    funder_id_field: "metadata.nested.funder_id",
                    drop_id_field: "metadata.nested.drop_id",
                    attached_deposit: NEAR.parse("1").toString(),
                }
            ]
        ]
    }

    const config: DropConfig = { 
        uses_per_key: 1
    }

    let {keys, publicKeys} = await generateKeyPairs(1);
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
    await keypom.setKey(keys[0]);

    // This should pass and none of the user provided args should be used.
    const res = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
    displayFailureLog(res);

    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].metadata.nested.account_id, bob.accountId);
    t.is(aliTokens[0].metadata.nested.funder_id, ali.accountId);
    t.is(aliTokens[0].metadata.nested.key_id, "0");
    t.is(aliTokens[0].metadata.nested.drop_id, "0");
});

test('Nested Fields that Dont Exist', async t => {
    const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

    const fcData: FCData = {
        methods: [
            [
                {
                    receiver_id: nftContract.accountId,
                    method_name: 'nft_mint',
                    args: JSON.stringify({
                        receiver_id: ali.accountId,
                        token_id: '1',
                    }),
                    account_id_field: "metadata.nested.account_id",
                    key_id_field: "metadata.nested.key_id",
                    funder_id_field: "metadata.nested.funder_id",
                    drop_id_field: "metadata.nested.drop_id",
                    attached_deposit: NEAR.parse("1").toString(),
                }
            ]
        ]
    }

    const config: DropConfig = { 
        uses_per_key: 1
    }

    let {keys, publicKeys} = await generateKeyPairs(1);
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
    await keypom.setKey(keys[0]);

    // This should pass and none of the user provided args should be used.
    const res = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
    displayFailureLog(res);

    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].metadata.nested.account_id, bob.accountId);
    t.is(aliTokens[0].metadata.nested.funder_id, ali.accountId);
    t.is(aliTokens[0].metadata.nested.key_id, "0");
    t.is(aliTokens[0].metadata.nested.drop_id, "0");
});

test('Nested Fields That Are Not an Object', async t => {
    const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

    const fcData: FCData = {
        methods: [
            [
                {
                    receiver_id: nftContract.accountId,
                    method_name: 'nft_mint',
                    args: JSON.stringify({
                        receiver_id: ali.accountId,
                        token_id: '1',
                        metadata: {}
                    }),
                    account_id_field: "token_id.account_id",
                    key_id_field: "token_id.key_id",
                    funder_id_field: "token_id.funder_id",
                    drop_id_field: "token_id.drop_id",
                    attached_deposit: NEAR.parse("1").toString(),
                }
            ]
        ]
    }

    const config: DropConfig = { 
        uses_per_key: 1
    }

    let {keys, publicKeys} = await generateKeyPairs(1);
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
    await keypom.setKey(keys[0]);

    // This should pass and none of the user provided args should be used.
    await keypom.call(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
    //displayFailureLog(res);

    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 0);
});