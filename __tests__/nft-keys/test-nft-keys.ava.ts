import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, getDropInformation, getKeyInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, NFTTokenObject, SimpleData, TokenMetadata } from "../utils/types";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
  }>;

  test.beforeEach(async (t) => {
  // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy the keypom contract.
    const keypom = await root.devDeploy(`./out/keypom.wasm`);

    // Init the contract
    await keypom.call(keypom, 'new', {root_account: 'testnet', owner_id: keypom, contract_metadata: CONTRACT_METADATA});

    // Test users
    const ali = await root.createSubAccount('ali');
    const bob = await root.createSubAccount('bob');

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Create drop with 1 NFT (no owner)', async t => {
    const { keypom, ali } = t.context.accounts;
    //add 2NEAR to ali's keypom balance
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    const {publicKeys} = await generateKeyPairs(1);
    //create a drop with Ali, doesn't front any cost. 
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('5 mN').toString(), config: {nft_key_behaviour: {nft_metadata: {title: "My Cool NFT!", media: 'bafybeibwhlfvlytmttpcofahkukuzh24ckcamklia3vimzd4vkgnydy7nq'}}}});

    const tokens: NFTTokenObject[] = await keypom.view('nft_tokens');
    console.log('tokens: ', tokens)
    t.is(tokens.length, 1)
    t.is(tokens[0].owner_id, keypom.accountId)
    t.is(tokens[0].metadata.title, "My Cool NFT!")

    let tokensForOwner: NFTTokenObject[] = await keypom.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('tokensForOwner (ali): ', tokensForOwner)
    t.is(tokensForOwner.length, 0);

    tokensForOwner = await keypom.view('nft_tokens_for_owner', {account_id: keypom.accountId});
    console.log('tokensForOwner (keypom): ', tokensForOwner)
    t.is(tokensForOwner.length, 1);
});

test('Create drop with 1 NFT (with owner)', async t => {
    const { keypom, ali } = t.context.accounts;
    //add 2NEAR to ali's keypom balance
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    const {publicKeys} = await generateKeyPairs(1);
    //create a drop with Ali, doesn't front any cost. 
    await ali.call(keypom, 'create_drop', {public_keys: publicKeys, key_owners: [ali.accountId], deposit_per_use: NEAR.parse('5 mN').toString(), config: {nft_key_behaviour: {nft_metadata: {title: "My Cool NFT!", media: 'bafybeibwhlfvlytmttpcofahkukuzh24ckcamklia3vimzd4vkgnydy7nq'}}}});

    const tokens: NFTTokenObject[] = await keypom.view('nft_tokens');
    console.log('tokens: ', tokens)
    t.is(tokens.length, 1)
    t.is(tokens[0].owner_id, ali.accountId)
    t.is(tokens[0].metadata.title, "My Cool NFT!")

    let tokensForOwner: NFTTokenObject[] = await keypom.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('tokensForOwner (ali): ', tokensForOwner)
    t.is(tokensForOwner.length, 1);

    tokensForOwner = await keypom.view('nft_tokens_for_owner', {account_id: keypom.accountId});
    console.log('tokensForOwner (keypom): ', tokensForOwner)
    t.is(tokensForOwner.length, 0);
});

test('Add keys to drop with owner', async t => {
    const { keypom, ali } = t.context.accounts;
    //add 2NEAR to ali's keypom balance
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    const {publicKeys} = await generateKeyPairs(2);
    //create a drop with Ali, doesn't front any cost. 
    await ali.call(keypom, 'create_drop', {public_keys: [publicKeys[0]], deposit_per_use: NEAR.parse('5 mN').toString(), config: {nft_key_behaviour: {nft_metadata: {title: "My Cool NFT!", media: 'bafybeibwhlfvlytmttpcofahkukuzh24ckcamklia3vimzd4vkgnydy7nq'}}}});
    await ali.call(keypom, 'add_keys', {drop_id: '0', public_keys: [publicKeys[1]], key_owners: [ali.accountId]});

    let tokensForOwner: NFTTokenObject[] = await keypom.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('tokensForOwner (ali): ', tokensForOwner)
    t.is(tokensForOwner.length, 1);

    tokensForOwner = await keypom.view('nft_tokens_for_owner', {account_id: keypom.accountId});
    console.log('tokensForOwner (keypom): ', tokensForOwner)
    t.is(tokensForOwner.length, 1);
});

test('Transfer key to new user', async t => {
    const { keypom, ali } = t.context.accounts;
    //add 2NEAR to ali's keypom balance
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    const {publicKeys} = await generateKeyPairs(2);
    //create a drop with Ali, doesn't front any cost. 
    await ali.call(keypom, 'create_drop', {public_keys: [publicKeys[0]], deposit_per_use: NEAR.parse('5 mN').toString(), config: {nft_key_behaviour: {nft_metadata: {title: "My Cool NFT!", media: 'bafybeibwhlfvlytmttpcofahkukuzh24ckcamklia3vimzd4vkgnydy7nq'}}}});
    await ali.call(keypom, 'add_keys', {drop_id: '0', public_keys: [publicKeys[1]], key_owners: [ali.accountId]});

    let tokensForOwner: NFTTokenObject[] = await keypom.view('nft_tokens_for_owner', {account_id: ali.accountId});
    console.log('tokensForOwner (ali): ', tokensForOwner)
    t.is(tokensForOwner.length, 1);

    tokensForOwner = await keypom.view('nft_tokens_for_owner', {account_id: keypom.accountId});
    console.log('tokensForOwner (keypom): ', tokensForOwner)
    t.is(tokensForOwner.length, 1);
});