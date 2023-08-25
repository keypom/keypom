import anyTest, { TestFn } from "ava";
import { KeyPairEd25519, NEAR, NearAccount, Worker } from "near-workspaces";
import { assertBalanceChange, CONTRACT_METADATA, defaultCallOptions, DEFAULT_DEPOSIT, generateKeyPairs, functionCall } from "../utils/general";

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
    t.context.accounts = { root, funder, keypomV3 };
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Changing linkdrop contract', async t => {
    //get default root account for this environment, which should be testnet as defined in config file
    const { root, keypomV3 } = t.context.accounts;
    let result = await keypomV3.view('get_root_account', {});
    t.is(result, root.accountId);

    //change root account to foo
    await keypomV3.call(keypomV3, 'set_root_account', {root_account: 'foo'});
    
    //ensure that new root account is foo
    result = await keypomV3.view('get_root_account', {});
    t.is(result, 'foo');
});

test('Setting Contract Metadata', async t => {
    //ensure that default contract metadata is equal to CONTRACT_METADATA
    const { keypomV3 } = t.context.accounts;
    let result = await keypomV3.view('contract_source_metadata', {});
    t.deepEqual(result, {version: "3.0.0", link: "hello"});

    //create new set of metadata
    let newMetadata = {
        "version": "0.0.1",
        "link": "foo"
    }
    //set new metadata as contract metadata and ensure that contract metadata has actually changed to new metadata
    await keypomV3.call(keypomV3, 'set_contract_metadata', {contract_metadata: newMetadata});
    
    result = await keypomV3.view('contract_source_metadata', {});
    t.deepEqual(result, newMetadata);
});

test('Deposit & withdraw to user balance', async t => {
    //set up keypom and funder accounts, make sure funder's initial balance is 0
    const { keypomV3, funder } = t.context.accounts;
    let result = await keypomV3.view('get_user_balance', {account_id: funder});
    t.is(result, NEAR.parse("10").toString());

    //add balance to funder's account, default balance in general.ts has default gas and default deposit values
    let b1 = await funder.availableBalance();
    //add_to_balance decrements NEAR wallet and adds it to Keypom wallet. This should give us a balance difference in the NEAR wallet equal to the Keypom balance added
    await funder.call(keypomV3, 'add_to_balance', {}, defaultCallOptions());
    let b2 = await funder.availableBalance();
    //assert that balance change is equal to default deposit vals
    t.assert(assertBalanceChange(b1, b2, new NEAR(DEFAULT_DEPOSIT), 0.01), "balance didn't decrement properly with 1% precision");

    //double check funder's balance is default deposit
    result = await keypomV3.view('get_user_balance', {account_id: funder});
    // DEFAULT_DEPOSIT + 10N == 11N
    t.is(result, NEAR.parse("11").toString());

    //withdraw default balance from funder's account using withdraw_from_balance. change in balance should be default_deposit
    b1 = await funder.availableBalance();
    await funder.call(keypomV3, 'withdraw_from_balance', {});
    b2 = await funder.availableBalance();
    t.assert(assertBalanceChange(b1, b2, NEAR.parse("11"), 0.01), "balance didn't increment properly with 1% precision");

    //funder's new balance should now be 0, return boolean
    result = await keypomV3.view('get_user_balance', {account_id: funder});
    t.is(result, '0');
});

test('Withdrawing fees earned', async t => {
    //check fees collected by the contract, should start at 0
    const { keypomV3, funder } = t.context.accounts;
    let result: string = await keypomV3.view('get_fees_collected', {});
    t.is(result, '0');
    //set drop fees and then give funder the balance necessary to create the drop
    await keypomV3.call(keypomV3, 'set_fees', {drop_fee: NEAR.parse("1").toString(), key_fee: NEAR.parse('5 mN').toString()});
    const dropId = "Null Claim";
    const asset_data = [
        {
            assets: [null],
            uses: 1
        }
    ]
    let keyPairs = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: []
        },
        attachedDeposit: NEAR.parse("10").toString()
    })    
    //verify that set drop fee succeeded, one drop created and thus 1 $NEAR should have been collected in fees
    result = await keypomV3.view('get_fees_collected', {});
    t.is(result, NEAR.parse("1").toString());
    
    //adding keys should cost 5mN
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data: [{
                public_key: keyPairs.publicKeys[0]
            }]
        },
    })
    //verify cost, total should now me 1.005N or 1005mN
    result = await keypomV3.view('get_fees_collected', {});
    t.is(result, NEAR.parse('1005 mN').toString());

    //try to withdraw fees to non-existent testnet account, remaining fees on the contract should stay the same 
    let withdraw_result = await keypomV3.call(keypomV3, 'withdraw_fees', {withdraw_to: "no-exist.testnet"});
    console.log(withdraw_result)
    t.is(withdraw_result, false)
    result = await keypomV3.view('get_fees_collected', {});
    t.is(result, NEAR.parse('1005 mN').toString());

    //withdraw to funder, should leave 0 balance on the contract; store b1 value as ref
    let b1 = await funder.availableBalance();
    withdraw_result = await keypomV3.call(keypomV3, 'withdraw_fees', {withdraw_to: funder});
    console.log(result)
    t.is(withdraw_result, true)
    result = await keypomV3.view('get_fees_collected', {});
    t.is(result, NEAR.parse('0').toString());

    //get funder's new balance and see if it has changed by 1005mN; this indicates that balance out of contract = balace into funder's account
    let b2 = await funder.availableBalance();
    t.assert(assertBalanceChange(b1, b2, NEAR.parse('1005 mN'), 0.01), "balance didn't decrement properly with 1% precision");
});

test('Custom fees earned', async t => {
    //init new account and contract, fees collected 0
    const { keypomV3, funder } = t.context.accounts;
    let result = await keypomV3.view('get_fees_collected', {});
    t.is(result, '0');
    
    //set drop fee to 5N and key fee to 0N. Add 10N to Ali's balance and create drop using Ali's account
    await keypomV3.call(keypomV3, 'set_fees_per_user', {account_id: funder, drop_fee: NEAR.parse("5").toString(), key_fee: "0"});
    const dropId = "Null Claim";
    const asset_data = [
        {
            assets: [null],
            uses: 1
        }
    ]
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data,
            key_data: []
        },
        attachedDeposit: NEAR.parse("10").toString()
    })      
    //make sure set_fees_per_user configured correctly and actually decremented 5N
    result = await keypomV3.view('get_fees_collected', {});
    t.is(result, NEAR.parse("5").toString());
    
    let keyPairs = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data: [{
                public_key: keyPairs.publicKeys[0]
            }]
        },
    })
    //fees collected should remain unchanged from before as key_fee = 0N
    result = await keypomV3.view('get_fees_collected', {});
    t.is(result, NEAR.parse('5').toString());
});