import anyTest, { TestFn } from "ava";
import { ExecutionStatusBasic, NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, displayAllLogs, displayFailureLog, generateKeyPairs, getDropInformation, getKeyInformation, LARGE_GAS, WALLET_GAS } from "../utils/general";
import { DropConfig, FCData } from "../utils/types";
const { readFileSync } = require('fs')

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
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    const nftContractNested = await root.devDeploy(`./__tests__/ext-wasm/nested-fields-nft.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });
    await nftContractNested.call(nftContractNested, 'new_default_meta', { owner_id: nftContractNested });

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const bob = await root.createSubAccount('bob');
    
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: owner.accountId });
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: ali.accountId });
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: bob.accountId });
    
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

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, nftContract, nftContractNested, owner, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

const TERA_GAS = 1000000000000;

// test('Attached Gas', async t => {
//     const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

//     const fcData1: FCData = {
//         methods: [
//             [
//                 {
//                     receiver_id: nftContract.accountId,
//                     method_name: 'nft_mint',
//                     args: "",
//                     attached_deposit: NEAR.parse("1").toString(),
//                     attached_gas: (20 * TERA_GAS).toString(),
//                 },
//                 {
//                     receiver_id: nftContract.accountId,
//                     method_name: 'nft_mint',
//                     args: "",
//                     attached_deposit: NEAR.parse("1").toString(),
//                 },
//                 {
//                     receiver_id: nftContract.accountId,
//                     method_name: 'nft_mint',
//                     args: "",
//                     attached_deposit: NEAR.parse("1").toString(),
//                 },
//                 {
//                     receiver_id: nftContract.accountId,
//                     method_name: 'nft_mint',
//                     args: "",
//                     attached_deposit: NEAR.parse("1").toString(),
//                 }
//             ],
//             [
//             {
//                 receiver_id: nftContract.accountId,
//                 method_name: 'nft_mint',
//                 args: "",
//                 attached_deposit: NEAR.parse("1").toString(),
//             },
//             {
//                 receiver_id: nftContract.accountId,
//                 method_name: 'nft_mint',
//                 args: "",
//                 attached_deposit: NEAR.parse("1").toString(),
//             },
//             {
//                 receiver_id: nftContract.accountId,
//                 method_name: 'nft_mint',
//                 args: "",
//                 attached_deposit: NEAR.parse("1").toString(),
//             }
//             ]
//         ]
//     }

//     let {keys, publicKeys} = await generateKeyPairs(1);
//     await ali.call(keypom, 'create_drop', {
//         public_keys: publicKeys, 
//         deposit_per_use: "0", 
//         fc: fcData1,
//         config: {
//             uses_per_key: 2,
//         }
//     }, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
//     const drop = await getDropInformation(keypom, "0");
//     t.is(drop.required_gas, (40 * TERA_GAS).toString());
// });

// test('All Funder Tests', async t => {
//     const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

//     const fcData: FCData = {
//         methods: [
//             [
//                 {
//                     receiver_id: nftContract.accountId,
//                     method_name: 'nft_mint',
//                     args: JSON.stringify({
//                         token_id: '1',
//                         metadata: {
//                             title: "foo"
//                         }
//                     }),
//                     account_id_field: "receiver_id",
//                     attached_deposit: NEAR.parse("1").toString(),
//                 }
//             ]
//         ]
//     }

//     const config: DropConfig = { 
//         uses_per_key: 10
//     }

//     let {keys, publicKeys} = await generateKeyPairs(1);
//     await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
//     await keypom.setKey(keys[0]);

//     // This should pass and none of the user provided args should be used.
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({keypom_args: {account_id_field: "foo"}})]}, {gas: WALLET_GAS});
//     let bobSupply = await nftContract.view('nft_supply_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobSupply)
//     t.is(bobSupply, '1');
    
//     // This should fail since number of fc args is not equal to number of methods
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({keypom_args: {account_id_field: "foo"}}), JSON.stringify({keypom_args: {account_id_field: "foo"}})]}, {gas: WALLET_GAS});
//     bobSupply = await nftContract.view('nft_supply_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobSupply)
//     t.is(bobSupply, '1');
// });

// test('User Preferred Tests', async t => {
//     const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

//     const fcData: FCData = {
//         methods: [
//             [
//                 {
//                     receiver_id: nftContract.accountId,
//                     method_name: 'nft_mint',
//                     args: JSON.stringify({
//                         token_id: '1',
//                         metadata: {}
//                     }),
//                     user_args_rule: "UserPreferred",
//                     account_id_field: "receiver_id",
//                     attached_deposit: NEAR.parse("1").toString(),
//                 }
//             ]
//         ]
//     }

//     const config: DropConfig = { 
//         uses_per_key: 10
//     }

//     let {keys, publicKeys} = await generateKeyPairs(1);
//     await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
//     await keypom.setKey(keys[0]);

//     // Should go through with token ID equal to 1
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
//     let bobTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobTokens)
//     t.is(bobTokens[0].token_id, '1');

//     // Token ID should be replaced with 2
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({token_id: "2"})]}, {gas: WALLET_GAS});
//     bobTokens = await nftContract.view('nft_tokens_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobTokens)
//     t.is(bobTokens[1].token_id, '2');

//     // Token ID should be replaced with 3 and metadata should now be included
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({token_id: "3", metadata: {title: "i injected this"}})]}, {gas: WALLET_GAS});
//     bobTokens = await nftContract.view('nft_tokens_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobTokens);
//     t.is(bobTokens[2].token_id, '3');
//     t.is(bobTokens[2].metadata.title, "i injected this");

//     // Receiver ID should be overwritten to bob even though bob passed in ali
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({token_id: "4", receiver_id: ali.accountId})]}, {gas: WALLET_GAS});
//     bobTokens = await nftContract.view('nft_tokens_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobTokens);
//     t.is(bobTokens.length, 4);

//     const aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
//     console.log('aliTokens: ', aliTokens);
//     t.is(aliTokens.length, 0);
// });

// test('Funder Preferred Tests', async t => {
//     const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

//     const fcData: FCData = {
//         methods: [
//             [
//                 {
//                     receiver_id: nftContract.accountId,
//                     method_name: 'nft_mint',
//                     args: JSON.stringify({
//                         metadata: {
//                             title: "this was here"
//                         }
//                     }),
//                     user_args_rule: "FunderPreferred",
//                     account_id_field: "receiver_id",
//                     attached_deposit: NEAR.parse("1").toString(),
//                 }
//             ]
//         ]
//     }

//     const config: DropConfig = { 
//         uses_per_key: 10
//     }

//     let {keys, publicKeys} = await generateKeyPairs(1);
//     await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
//     await keypom.setKey(keys[0]);

//     // Should go through with token ID equal to 1
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({token_id: "1"})]}, {gas: WALLET_GAS});
//     let bobTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobTokens)
//     t.is(bobTokens[0].token_id, '1');

//     // metadata should not be replaced
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({token_id: "2", metadata: {title: "i injected this"}})]}, {gas: WALLET_GAS});
//     bobTokens = await nftContract.view('nft_tokens_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobTokens)
//     t.is(bobTokens[1].token_id, '2');
//     t.is(bobTokens[1].metadata.title, "this was here");

//     // metadata should have appended fields
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({token_id: "3", metadata: {title: "i injected this", description: "i injected this"}})]}, {gas: WALLET_GAS});
//     bobTokens = await nftContract.view('nft_tokens_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobTokens)
//     t.is(bobTokens[2].token_id, '3');
//     t.is(bobTokens[2].metadata.title, "this was here");
//     t.is(bobTokens[2].metadata.description, "i injected this");

//     // Receiver ID should be overwritten to bob even though bob passed in ali
//     await keypom.call(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify({token_id: "4", receiver_id: ali.accountId})]}, {gas: WALLET_GAS});
//     bobTokens = await nftContract.view('nft_tokens_for_owner', {account_id: bob.accountId});
//     console.log('bobSupply: ', bobTokens);
//     t.is(bobTokens.length, 4);

//     const aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
//     console.log('aliTokens: ', aliTokens);
//     t.is(aliTokens.length, 0);
// });

test('User Marker Tests', async t => {
    const { keypom, nftContractNested: nftContract, owner, ali, bob } = t.context.accounts;

    const fcData: FCData = {
        methods: [
            [
                {
                    receiver_id: nftContract.accountId,
                    method_name: 'nft_mint',
                    args: JSON.stringify({
                        receiver_id: "INSERT_RECEIVER_ID",
                        token_id: 'lower_case',
                        metadata: {
                            title: "INSERT_TITLE",
                            description: "INSERT_DESCRIPTION",
                            nested: "INSERT_NESTED"
                        },
                        long_args: [...readFileSync(`./__tests__/ext-wasm/nested-fields-nft.wasm`)].slice(0, 5000)
                    }),
                    attached_deposit: NEAR.parse("1").toString(),
                    user_args_rule: "UserPreferred"
                }
            ]
        ]
    }

    let {keys, publicKeys} = await generateKeyPairs(1);
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
    await keypom.setKey(keys[0]);

    let fcArgs = {
        "lower_case": "inserted token id",
        "INSERT_RECEIVER_ID": ali.accountId,
        "INSERT_TITLE": "inserted title",
        "INSERT_DESCRIPTION": "inserted description",
        "INSERT_NESTED": {
            "account_id": bob.accountId,
            "key_id": "0",
            "funder_id": ali.accountId,
            "drop_id": "0"
        }
    }
    // This should pass and none of the user provided args should be used.
    const res = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId, fc_args: [JSON.stringify(fcArgs)]}, {gas: WALLET_GAS});
    displayFailureLog(res);
    displayAllLogs(res);
    let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('aliTokens: ', aliTokens)
    t.is(aliTokens.length, 1);
    t.is(aliTokens[0].token_id, "lower_case");
    t.is(aliTokens[0].metadata.title, "inserted title");
    t.is(aliTokens[0].metadata.description, "inserted description");
    t.is(aliTokens[0].metadata.nested.account_id, bob.accountId);
    t.is(aliTokens[0].metadata.nested.funder_id, ali.accountId);
    t.is(aliTokens[0].metadata.nested.key_id, "0");
    t.is(aliTokens[0].metadata.nested.drop_id, "0");
});