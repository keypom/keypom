import anyTest, { TestFn } from "ava";
import { Account, NEAR, NearAccount, Worker } from "near-workspaces";
import { generateKeyPairs, LARGE_GAS, WALLET_GAS } from "../utils/general";
import { JsonKeyInfo } from "../utils/types";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
}>;

const ONE_SECOND_NS = 1e9;

test.beforeEach(async (t) => {
    // Comment this if you want to see console logs
    //console.log = function() {}

    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy all 3 contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom });
    
    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    
    // Custom root
    const customRoot = await root.createSubAccount('custom-root');
    await customRoot.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    await customRoot.call(customRoot, 'new', {});

    // Add 10k $NEAR to owner's account
    await owner.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, owner, ali, customRoot };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Testing Delete On Empty Config', async t => {
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(2);
    let config = {
        delete_on_empty: true,
    }

    // Creating the drop that should be deleted
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Creating the drop that should NOT be deleted
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[1]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config: null,
    },{gas: LARGE_GAS});

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    for(var i = 0; i < 2; i++) {
        await keypom.setKey(keys[i]);
        await keypom.updateAccessKey(
            publicKeys[i],  // public key
            {
                nonce: 0,
                permission: 'FullAccess'
            }
        )
        await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    }

    let aliBal = await ali.availableBalance();
    console.log('aliBal: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());

    const dropSupplyForOwner = await keypom.view('get_drop_supply_for_owner', {account_id: owner.accountId});
    console.log('dropSupplyForOwner: ', dropSupplyForOwner)
    t.is(dropSupplyForOwner, 1);

    const getKeySupplyForDrop = await keypom.view('get_key_supply_for_drop', {drop_id: 1});
    console.log('getKeySupplyForDrop: ', getKeySupplyForDrop)
    t.is(getKeySupplyForDrop, 0);
});

test('Testing Start Timestamp', async t => {
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(1);
    let config = {
        start_timestamp: (Date.now() * 1000000) + ONE_SECOND_NS * 30,
    }

    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // THIS SHOULD FAIL BECAUSE THE START TIMESTAMP HASN'T BEEN REACHED
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    let getKeySupplyForDrop = await keypom.view('get_key_supply_for_drop', {drop_id: 0});
    console.log('getKeySupplyForDrop: ', getKeySupplyForDrop)
    t.is(getKeySupplyForDrop, 1);

    // Wait 30 seconds and call the claim method again
    await new Promise(r => setTimeout(r, 30000));
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    getKeySupplyForDrop = await keypom.view('get_key_supply_for_drop', {drop_id: 0});
    console.log('getKeySupplyForDrop: ', getKeySupplyForDrop)
    t.is(getKeySupplyForDrop, 0);

    aliBal = await ali.availableBalance();
    console.log('aliBal After: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());
});

test('Testing Throttle Timestamp', async t => {
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(1);
    let config = {
        uses_per_key: 2,
        throttle_timestamp: ONE_SECOND_NS * 30,
    }

    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // THIS SHOULD FAIL BECAUSE THE START TIMESTAMP HASN'T BEEN REACHED
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    let getKeySupplyForDrop = await keypom.view('get_key_supply_for_drop', {drop_id: 0});
    console.log('getKeySupplyForDrop: ', getKeySupplyForDrop)
    t.is(getKeySupplyForDrop, 1);

    let keyInformation: JsonKeyInfo = await keypom.view('get_key_information', {key: publicKeys[0]});
    console.log('keyInformation: ', keyInformation)
    t.is(keyInformation.key_info.remaining_uses, 1);

    // Wait 30 seconds and call the claim method again
    await new Promise(r => setTimeout(r, 30000));
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    getKeySupplyForDrop = await keypom.view('get_key_supply_for_drop', {drop_id: 0});
    console.log('getKeySupplyForDrop: ', getKeySupplyForDrop)
    t.is(getKeySupplyForDrop, 0);

    aliBal = await ali.availableBalance();
    console.log('aliBal After: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());
});

test('Testing On Claim Refund Deposit', async t => {
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(2);
    let config = {
        on_claim_refund_deposit: true,
    }

    // Creating the drop that should be deleted
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })
    // Withdrawing all owner's balance
    await owner.call(keypom, 'withdraw_from_balance', {});

    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    let aliBal = await ali.availableBalance();
    console.log('aliBal: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    let userBal: String = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.assert(userBal > NEAR.parse("1").toString())

    const dropSupplyForOwner = await keypom.view('get_drop_supply_for_owner', {account_id: owner.accountId});
    console.log('dropSupplyForOwner: ', dropSupplyForOwner)
    t.is(dropSupplyForOwner, 1);

    const getKeySupplyForDrop = await keypom.view('get_key_supply_for_drop', {drop_id: 0});
    console.log('getKeySupplyForDrop: ', getKeySupplyForDrop)
    t.is(getKeySupplyForDrop, 0);
});

test('Testing Custom Drop Root', async t => {
    const { keypom, owner, ali, customRoot } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(2);
    let config = {
        drop_root: customRoot.accountId,
    }

    // Creating the drop that points to the custom root
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});
    
    // Creating the regular drop
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[1]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config: null,
    },{gas: LARGE_GAS});

    let { keys: keys2, publicKeys: pks2 } = await generateKeyPairs(1);
    let newAccount = await keypom.getAccount(`foo.${customRoot.accountId}.test.near`);
    
    await keypom.setKey(keys[1]);
    await keypom.updateAccessKey(
        publicKeys[1],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // SHOULD NOT WORK
    await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `foo.${customRoot.accountId}.test.near`, new_public_key : pks2[0]}, {gas: WALLET_GAS});
    let doesExist = await newAccount.exists();
    t.is(doesExist, false);

    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `foo.${customRoot.accountId}.test.near`, new_public_key : pks2[0]}, {gas: WALLET_GAS});
    doesExist = await newAccount.exists();
    t.is(doesExist, false);
});