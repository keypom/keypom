import anyTest, { TestFn } from "ava";
import { KeyPairEd25519, NEAR, NearAccount, Worker } from "near-workspaces";
import { assertBalanceChange, CONTRACT_METADATA, defaultCallOptions, DEFAULT_DEPOSIT } from "../utils/general";

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

    // Create users
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

test('Initial nonce is 0', async t => {
    const { keypom } = t.context.accounts;
    //first drop id should be 0
    const result = await keypom.view('get_next_drop_id', {});
    t.is(result, 0);
});

test('Changing linkdrop contract', async t => {
    //get default root account for this environment, which should be testnet as defined in config file
    const { keypom } = t.context.accounts;
    let result = await keypom.view('get_root_account', {});
    t.is(result, 'testnet');

    //change root account to foo
    await keypom.call(keypom, 'set_root_account', {root_account: 'foo'});
    
    //ensure that new root account is foo
    result = await keypom.view('get_root_account', {});
    t.is(result, 'foo');
});

test('Setting Contract Metadata', async t => {
    //ensure that default contract metadata is equal to CONTRACT_METADATA
    const { keypom } = t.context.accounts;
    let result = await keypom.view('contract_source_metadata', {});
    t.deepEqual(result, CONTRACT_METADATA);

    //create new set of metadata
    let newMetadata = {
        "version": "0.0.1",
        "link": "foo"
    }
    //set new metadata as contract metadata and ensure that contract metadata has actually changed to new metadata
    await keypom.call(keypom, 'set_contract_metadata', {contract_metadata: newMetadata});
    
    result = await keypom.view('contract_source_metadata', {});
    t.deepEqual(result, newMetadata);
});

test('Setting gas price', async t => {
    //ensure gas price is default
    const { keypom } = t.context.accounts;
    let result = await keypom.view('get_gas_price', {});
    t.is(result, 100000000);

    //change to new gas price and check if the change succeeded. returns boolean
    await keypom.call(keypom, 'set_gas_price', {yocto_per_gas: 100});
    
    result = await keypom.view('get_gas_price', {});
    t.is(result, 100);
});

test('Deposit & withdraw to user balance', async t => {
    //set up keypom and ali accounts, make sure ali's initial balance is 0
    const { keypom, ali } = t.context.accounts;
    let result = await keypom.view('get_user_balance', {account_id: ali});
    t.is(result, '0');

    //add balance to ali's account, default balance in general.ts has default gas and default deposit values
    let b1 = await ali.availableBalance();
    //add_to_balance decrements NEAR wallet and adds it to Keypom wallet. This should give us a balance difference in the NEAR wallet equal to the Keypom balance added
    await ali.call(keypom, 'add_to_balance', {}, defaultCallOptions());
    let b2 = await ali.availableBalance();
    //assert that balance change is equal to default deposit vals
    t.assert(assertBalanceChange(b1, b2, new NEAR(DEFAULT_DEPOSIT), 0.01), "balance didn't decrement properly with 1% precision");

    //double check ali's balance is default deposit
    result = await keypom.view('get_user_balance', {account_id: ali});
    t.is(result, DEFAULT_DEPOSIT);

    //withdraw default balance from ali's account using withdraw_from_balance. change in balance should be default_deposit
    b1 = await ali.availableBalance();
    await ali.call(keypom, 'withdraw_from_balance', {});
    b2 = await ali.availableBalance();
    t.assert(assertBalanceChange(b1, b2, new NEAR(DEFAULT_DEPOSIT), 0.01), "balance didn't increment properly with 1% precision");

    //ali's new balance should now be 0, return boolean
    result = await keypom.view('get_user_balance', {account_id: ali});
    t.is(result, '0');
});

test('Withdrawing fees earned', async t => {
    //check fees collected by the contract, should start at 0
    const { keypom, ali } = t.context.accounts;
    let result = await keypom.view('get_fees_collected', {});
    t.is(result, '0');
    //set drop fees and then give ali the balance necessary to create the drop
    await keypom.call(keypom, 'set_fees', {drop_fee: NEAR.parse("1").toString(), key_fee: NEAR.parse('5 mN').toString()});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("2").toString()});
    await ali.call(keypom, 'create_drop', {deposit_per_use: NEAR.parse('5 mN').toString()})
    
    //verify that set drop fee succeeded, balance - drop fee = 2-1 = 1
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse("1").toString());
    
    //adding keys should cost 5mN
    let keyPair = await KeyPairEd25519.fromRandom();
    await ali.call(keypom, 'add_keys', {public_keys: [keyPair.publicKey.toString()], drop_id: '0'})

    //verify cost, total should now me 1.005N or 1005mN
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse('1005 mN').toString());

    //try to withdraw fees to non-existent testnet account, remaining fees on the contract should stay the same 
    result = await keypom.call(keypom, 'withdraw_fees', {withdraw_to: "no-exist.testnet"});
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse('1005 mN').toString());

    //withdraw to ali, should leave 0 balance on the contract; store b1 value as ref
    let b1 = await ali.availableBalance();
    result = await keypom.call(keypom, 'withdraw_fees', {withdraw_to: ali});
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse('0').toString());

    //get ali's new balance and see if it has changed by 1005mN; this indicates that balance out of contract = balace into ali's account
    let b2 = await ali.availableBalance();
    t.assert(assertBalanceChange(b1, b2, NEAR.parse('1005 mN'), 0.01), "balance didn't decrement properly with 1% precision");
});

test('Custom fees earned', async t => {
    //init new account and contract, fees collected 0
    const { keypom, ali } = t.context.accounts;
    let result = await keypom.view('get_fees_collected', {});
    t.is(result, '0');
    
    //set drop fee to 5N and key fee to 0N. Add 10N to Ali's balance and create drop using Ali's account
    await keypom.call(keypom, 'set_fees_per_user', {account_id: ali, drop_fee: NEAR.parse("5").toString(), key_fee: "0"});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});
    await ali.call(keypom, 'create_drop', {deposit_per_use: NEAR.parse('5 mN').toString()})
    
    //make sure set_fees_per_user configured correctly and actually decremented 5N
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse("5").toString());
    
    //adding keypairs should incur a 0N cost
    let keyPair = await KeyPairEd25519.fromRandom();
    await ali.call(keypom, 'add_keys', {public_keys: [keyPair.publicKey.toString()], drop_id: '0'})

    //fees collected should remain unchanged from before as key_fee = 0N
    result = await keypom.view('get_fees_collected', {});
    t.is(result, NEAR.parse('5').toString());
});