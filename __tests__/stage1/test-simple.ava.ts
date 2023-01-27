import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, getDropInformation, getKeyInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, SimpleData } from "../utils/types";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
  }>;

  test.beforeEach(async (t) => {
  // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    const nearDrop = await root.devDeploy(`./__tests__/ext-wasm/ext_linkdrop.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    await root.call(root, 'new', {});

    // Test users
    const ali = await root.createSubAccount('ali');
    const bob = await root.createSubAccount('bob');

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, nearDrop, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Attempt to Panic During Claim or CAAC', async t => {
    const { nearDrop, ali, root } = t.context.accounts;

    let {keys, publicKeys} = await generateKeyPairs(3);
    await nearDrop.call(nearDrop, 'create_user_account', {name: 'alice', public_key: publicKeys[0]}, {attachedDeposit: NEAR.parse("90").toString(), gas: LARGE_GAS});
    
    let alice = await nearDrop.getAccount(`alice.${nearDrop.accountId}`);
    await alice.setKey(keys[0]);
    await alice.call(alice, 'create_near_campaign', {name: "campaign", public_key: publicKeys[1], total_keys: 1, tokens_per_key: NEAR.parse("5").toString(), account_creator: root.accountId}, {attachedDeposit: NEAR.parse("50").toString(), gas: LARGE_GAS});
    
    let campaign = await alice.getAccount(`campaign.${alice.accountId}`);
    await campaign.setKey(keys[1]);
    await campaign.call(campaign, 'add_keys', {keys: [publicKeys[2]]}, {gas: LARGE_GAS});

    const foo = await campaign.view('get_campaign_metadata');
    console.log('foo: ', foo)

    let campaignBalance = await campaign.balance();
    console.log('campaign available INITIAL: ', campaignBalance.available.toString())
    console.log('campaign staked INITIAL: ', campaignBalance.staked.toString())
    console.log('campaign stateStaked INITIAL: ', campaignBalance.stateStaked.toString())
    console.log('campaign total INITIAL: ', campaignBalance.total.toString())

    await campaign.setKey(keys[2]);
    campaign.call(campaign, 'create_account_and_claim', {new_account_id: `foo.${root.accountId}`, new_public_key : publicKeys[0]}, {gas: WALLET_GAS});
    campaign.call(campaign, 'create_account_and_claim', {new_account_id: `foobar.${root.accountId}`, new_public_key : publicKeys[1]}, {gas: WALLET_GAS});

    // wait 10 seconds
    await new Promise(r => setTimeout(r, 10000));

    campaignBalance = await campaign.balance();
    console.log('campaign available AFTER: ', campaignBalance.available.toString())
    console.log('campaign staked AFTER: ', campaignBalance.staked.toString())
    console.log('campaign stateStaked AFTER: ', campaignBalance.stateStaked.toString())
    console.log('campaign total AFTER: ', campaignBalance.total.toString())

    let newAccountOne = await root.getAccount(`foo.test.near`);
    let newAccountTwo = await root.getAccount(`foobar.test.near`);
    let doesExistOne = await newAccountOne.exists();
    console.log('doesExistOne: ', doesExistOne)
    let doesExistTwo = await newAccountTwo.exists();
    console.log('doesExistTwo: ', doesExistTwo)

    const fooAfter = await campaign.view('get_campaign_metadata');
    console.log('fooAfter: ', fooAfter)
});