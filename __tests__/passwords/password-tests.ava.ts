import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, LARGE_GAS, WALLET_GAS } from "../utils/general";
import { generateLocalPasswords, hash } from "./utils/pwUtils";

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

    // Deploy both contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init the contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });

    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const bob = await root.createSubAccount('bob');

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, owner, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// test('Wrong number of passwords for each key', async t => {
// });

test('Multi-use keys with local passwords', async t => {
    // Comment this if you want to see console logs
    //console.log = function() {}
    
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config = {
        uses_per_key: 3,
    }

    let basePassword = 'mypassword1';
    let passwords = generateLocalPasswords(publicKeys, publicKeys, [0, 1, 2], basePassword);
    console.log('passwords: ', passwords)

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys, 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
        passwords_per_use: passwords
    },{gas: LARGE_GAS});

    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    // THIS SHOULD FAIL BECAUSE NO PASSWORD PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '0')}, {gas: WALLET_GAS});

    // THIS SHOULD FAIL
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '0')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '2')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("3").toString());
});

// test('Multi-use keys with global password', async t => {
// });

// test('Multi-use keys with local password some are none', async t => {
// });

// test('Some keys have a password and some do not', async t => {
// });

// test('Cheating with explorer', async t => {
// });
