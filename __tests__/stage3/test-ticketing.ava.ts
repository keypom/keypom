import { Worker, NearAccount, NEAR, KeyPairEd25519, tGas, KeyPair } from "near-workspaces";
import anyTest, { TestFn } from "ava";
import { assertBalanceChange, defaultCallOptions, DEFAULT_DEPOSIT, DEFAULT_GAS, generateKeyPairs, LARGE_GAS, queryAllViewFunctions } from "../utils/utils";
import { JsonDrop } from "../utils/types";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    keys: KeyPair[];
    publicKeys: string[];
  }>;

  test.beforeEach(async (t) => {
  // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy the keypom contract.
    const keypom = await root.devDeploy(`./out/main.wasm`);
    const nftSeries = await root.devDeploy(`./__tests__/ext-wasm/nft-series.wasm`);

    // Init the contract
    await keypom.call(keypom, 'new', {root_account: 'testnet', owner_id: keypom});
    await nftSeries.call(nftSeries, 'new_default_meta', {owner_id: nftSeries});

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const bob = await root.createSubAccount('bob');

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("8").toString()});
    
    let config = {
        uses_per_key: 3,
        on_claim_refund_deposit: true
    }

    let fc_data = {
        methods: [
            null,
            null,
            [{
                receiver_id: nftSeries,
                method_name: "nft_mint",
                args: "",
                attached_deposit: NEAR.parse("0.01").toString(),
                account_id_field: "receiver_id",
                drop_id_field: "id"
            }]
        ]
    }

    let {keys, publicKeys} = await generateKeyPairs(100);
    
    console.log("creating drop");
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys, 
        deposit_per_use: NEAR.parse('5 mN').toString(),
        fc_data,
        config,
    },{gas: LARGE_GAS});

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, nftSeries, owner, ali, bob};
    t.context.keys = keys;
    t.context.publicKeys = publicKeys;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Stage 1 Test views', async t => {
    const { keypom, nftSeries, owner, ali, bob} = t.context.accounts;
    const keys = t.context.keys;
    const publicKeys = t.context.publicKeys;

    console.log("querying views");
    let result = await queryAllViewFunctions({
        contract: keypom, 
        drop_id: 0, 
        account_id: owner.accountId,
        key: publicKeys[0]
    });

    t.is(result.keyBalance, NEAR.parse('5 mN').toString());
    let jsonKeyInfo = result.keyInformation;
    console.log('jsonKeyInfo: ', jsonKeyInfo)
    t.is(jsonKeyInfo?.drop_id, 0);
    t.is(jsonKeyInfo?.pk, publicKeys[0]);
    let keyInfo = jsonKeyInfo?.key_info;
    console.log('keyInfo: ', keyInfo)
    t.is(keyInfo?.key_id, 0);
    t.is(keyInfo?.remaining_uses, 3);

    let jsonDrop = result.dropInformation!;
    console.log('jsonDrop: ', jsonDrop)
    t.is(jsonDrop.drop_id, 0);
    t.is(jsonDrop.owner_id, owner.accountId);
    t.is(jsonDrop.deposit_per_use, NEAR.parse('5 mN').toString());
    
    let dropType = jsonDrop.drop_type.FunctionCall;
    console.log('dropType: ', dropType)

    //t.is(jsonDrop.drop_type, 'FunctionCall');
    t.deepEqual(jsonDrop.config, {
        uses_per_key: 3,
        start_timestamp: null,
        throttle_timestamp: null,
        on_claim_refund_deposit: true,
        claim_permission: null,
        drop_root: null,
    });
    t.is(jsonDrop.metadata, null);
    t.is(jsonDrop.registered_uses, 300);
    t.is(jsonDrop.required_gas, tGas(100));
    t.is(jsonDrop.next_key_id, 100);

    t.is(result.keySupplyForDrop, 100);
    t.is(result.keyTotalSupply, '100');

    t.deepEqual(result.dropSupplyForOwner, 1);
});

test('Fully Claim 1 key', async t => {
    const { keypom, nftSeries, owner, ali, bob} = t.context.accounts;
    const keys = t.context.keys;
    const publicKeys = t.context.publicKeys;

    bob.updateAccount({
        amount: "0"
    });

    await keypom.call(keypom, 'claim', {account_id: bob});
    await keypom.call(keypom, 'claim', {account_id: bob});
    await keypom.call(keypom, 'claim', {account_id: bob});

    console.log("querying views");
    let result = await queryAllViewFunctions({
        contract: keypom, 
        drop_id: 0, 
        account_id: owner.accountId,
        key: publicKeys[0]
    });
});

// test('Ticketing Injected Fields', async t => {
// });