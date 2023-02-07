import anyTest, { TestFn } from "ava";
import { parseNearAmount } from "near-api-js/lib/utils/format";
import { NEAR, NearAccount, parseNEAR, Worker } from "near-workspaces";
import { oneGtNear, totalSupply } from "../ft-drops/utils/ft-utils";
import { assertBalanceChange, CONTRACT_METADATA, generateKeyPairs, getDropInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, FTData } from "../utils/types";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    keypomInitialBalance: NEAR;
    keypomInitialStateStaked: NEAR;
}>;

const ONE_SECOND_NS = 1e9;

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

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const bob = await root.createSubAccount('bob');
    const eve = await root.createSubAccount('eve');
    
    // Add 10k $NEAR to owner's account
    await owner.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: owner.accountId });
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: ali.accountId });
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: bob.accountId });
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: eve.accountId });
    
    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, owner, ali, bob, eve };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Pub Sale No Reqs', async t => {
    //create account and add balance of 10NEAR to owner
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair and create drop config to have 2 uses/key and a throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config: DropConfig = {
        sale: {}
    }

    //create a drop with the created config
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    await owner.call(keypom, 'withdraw_from_balance', {});
    // Set owner's NEAR wallet balance to 0 so we can check if the claim works properly
    await owner.updateAccount({
        amount: "0"
    })

    // Open to the public therefore ali can add keys
    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[0]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});

    //owner should have 0 NEAR since the sale was free
    let ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    t.is(ownerBal.toString(), "0");

    //owner's Keypom wallet balance should be 0 since the sale was free
    let userBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.is(userBal, "0");

    //should still have 1 key as the single key that was added to the drop has two uses
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);

    let dropInfo = await getDropInformation(keypom, "0");
    console.log('dropInfo: ', dropInfo)
    t.assert(dropInfo.config?.sale != undefined);
});

test('Timed Sale', async t => {
    //create account and add balance of 10NEAR to owner
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair and create drop config to have 2 uses/key and a throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(2);
    let config: DropConfig = {
        sale: {
            start: (Date.now() * 1000000) + ONE_SECOND_NS * 15,
            end: (Date.now() * 1000000) + ONE_SECOND_NS * 35,
        }
    }

    //create a drop with the created config
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    await owner.call(keypom, 'withdraw_from_balance', {});
    // Set owner's NEAR wallet balance to 0 so we can check if the claim works properly
    await owner.updateAccount({
        amount: "0"
    })

    // SHOULD FAIL since the start hasn't been reached yet
    try {
        await ali.call(keypom, 'add_keys', {
            public_keys: [publicKeys[0]], 
            drop_id: "0"
        },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});
    } catch(e) {}

    //owner should have 0 NEAR since the sale was free
    let ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    t.is(ownerBal.toString(), "0");

    //owner's Keypom wallet balance should be 0 since the sale was free
    let userBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.is(userBal, "0");

    //should still have 1 key as the single key that was added to the drop has two uses
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 0);

    // Wait 10 seconds
    await new Promise(resolve => setTimeout(resolve, 10000));
    // Should now pass since start has been reached
    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[0]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});

    //owner should have 0 NEAR since the sale was free
    ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    t.is(ownerBal.toString(), "0");

    //owner's Keypom wallet balance should be 0 since the sale was free
    userBal = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.is(userBal, "0");

    //should still have 1 key as the single key that was added to the drop has two uses
    keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);

    // Wait 30 seconds
    await new Promise(resolve => setTimeout(resolve, 30000));
    // SHOULD FAIL since the end has been reached
    try {
        await ali.call(keypom, 'add_keys', {
            public_keys: [publicKeys[1]], 
            drop_id: "0"
        },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});
    } catch(e) {}

    //owner should have 0 NEAR since the sale was free
    ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    t.is(ownerBal.toString(), "0");

    //owner's Keypom wallet balance should be 0 since the sale was free
    userBal = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.is(userBal, "0");

    //should still have 1 key as the single key that was added to the drop has two uses
    keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);
});

test('Allowlist & Blocklist / Both', async t => {
    //create account and add balance of 10NEAR to owner
    const { keypom, owner, ali, bob, eve } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair and create drop config to have 2 uses/key and a throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(2);
    let config: DropConfig = {
        sale: {
            allowlist: [ali.accountId],
            blocklist: [bob.accountId]
        }
    }

    //create a drop with the created config
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    await owner.call(keypom, 'withdraw_from_balance', {});
    // Set owner's NEAR wallet balance to 0 so we can check if the claim works properly
    await owner.updateAccount({
        amount: "0"
    })

    // SHOULD FAIL since bob is in blocklist
    try {
        await bob.call(keypom, 'add_keys', {
            public_keys: [publicKeys[0]], 
            drop_id: "0"
        },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});
    } catch(e) {}

     // SHOULD FAIL since eve is NOT in allowlist
     try {
        await eve.call(keypom, 'add_keys', {
            public_keys: [publicKeys[0]], 
            drop_id: "0"
        },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});
    } catch(e) {}

    // Should pass since ali is in allowlist
    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[0]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});

    //owner should have 0 NEAR since the sale was free
    let ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    t.is(ownerBal.toString(), "0");

    //owner's Keypom wallet balance should be 0 since the sale was free
    let userBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.is(userBal, "0");

    //should still have 1 key as the single key that was added to the drop has two uses
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);
});

test('Auto Withdrawing Funds', async t => {
    //create account and add balance of 10NEAR to owner
    const { keypom, owner, ali, bob, eve } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair and create drop config to have 2 uses/key and a throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(2);
    let config: DropConfig = {
        sale: {
            auto_withdraw_funds: true,
            // @ts-ignore
            price_per_key: parseNearAmount("1")
        }
    }

    //create a drop with the created config
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    await owner.call(keypom, 'withdraw_from_balance', {});
    // Set owner's NEAR wallet balance to 0 so we can check if the claim works properly
    await owner.updateAccount({
        amount: "0"
    })

    // Should pass since no allowlist
    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[0]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});

    //owner should have 0 NEAR since the sale was free
    let ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    // @ts-ignore
    t.is(ownerBal.toString(), parseNearAmount("1"));

    //owner's Keypom wallet balance should be 0 since the sale was free
    let userBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.is(userBal, "0");

    //should still have 1 key as the single key that was added to the drop has two uses
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);
});

test('Max Number of Keys', async t => {
    //create account and add balance of 10NEAR to owner
    const { keypom, owner, ali, bob, eve } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair and create drop config to have 2 uses/key and a throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(3);
    let config: DropConfig = {
        sale: {
            // @ts-ignore
            price_per_key: parseNearAmount("1"),
            max_num_keys: 2
        }
    }

    //create a drop with the created config
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    await owner.call(keypom, 'withdraw_from_balance', {});
    // Set owner's NEAR wallet balance to 0 so we can check if the claim works properly
    await owner.updateAccount({
        amount: "0"
    })

    // Should pass since no allowlist
    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[0]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});

    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[1]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});

    // Should fail since max number of keys is 2
    try {
        await ali.call(keypom, 'add_keys', {
            public_keys: [publicKeys[2]], 
            drop_id: "0"
        },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});
    } catch(e){}

    //owner should have 0 NEAR since the sale was free
    let ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    // @ts-ignore
    t.is(ownerBal.toString(), "0");

    //owner's Keypom wallet balance should be 0 since the sale was free
    let userBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    // @ts-ignore
    t.is(userBal, parseNearAmount("2"));

    //should still have 1 key as the single key that was added to the drop has two uses
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 2);
});

test('Claiming Keys Check Refund Goes to Funder', async t => {
    //create account and add balance of 10NEAR to owner
    const { keypom, owner, ali, bob, eve } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair and create drop config to have 2 uses/key and a throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(3);
    let config: DropConfig = {
        sale: {
            // @ts-ignore
            price_per_key: parseNearAmount("5"),
        }
    }

    //create a drop with the created config
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    await owner.call(keypom, 'withdraw_from_balance', {});
    // Set owner's NEAR wallet balance to 0 so we can check if the claim works properly
    await owner.updateAccount({
        amount: "0"
    })

    // Should pass since no allowlist
    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[0]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});


    //owner should have 0 NEAR since the sale was free
    let ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    // @ts-ignore
    t.is(ownerBal.toString(), "0");

    //owner's Keypom wallet balance should be 0 since the sale was free
    let userBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    // @ts-ignore
    t.is(userBal, parseNearAmount("5"));


    //set access key to be used for following transactions
    await keypom.setKey(keys[0]);
    //give full access to the key above; used to circumvent allowance bug in sandbox (should be fixed now so this can be deleted)
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //owner should have 0 NEAR since the sale was free
    ownerBal = await owner.availableBalance();
    console.log('ownerBal Before: ', ownerBal.toString())
    // @ts-ignore
    t.is(ownerBal.toString(), "0");

    //owner's Keypom wallet balance should be 5 $NEAR plus refund amount
    userBal = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    // @ts-ignore
    t.assert(NEAR.from("5").lte(NEAR.parse(userBal)));
});

test('Update Sale Configs', async t => {
    //create account and add balance of 10NEAR to owner
    const { keypom, owner, ali, bob, eve } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair and create drop config to have 2 uses/key and a throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(2);
    let config: DropConfig = {
        sale: {
            allowlist: [ali.accountId],
        }
    }

    //create a drop with the created config
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // SHOULD FAIL since bob is in blocklist
    let canBobAddKeys = await keypom.view('can_user_add_keys', {account_id: bob.accountId, drop_id: "0"});
    t.is(canBobAddKeys, false);

    // Should pass since bob has been added to allowlist
    await owner.call(keypom, 'add_to_sale_allowlist', {drop_id: "0", account_ids: [bob.accountId]});
    canBobAddKeys = await keypom.view('can_user_add_keys', {account_id: bob.accountId, drop_id: "0"});
    t.is(canBobAddKeys, true);

    // Should fail since bob was removed to allowlost
    await owner.call(keypom, 'remove_from_sale_allowlist', {drop_id: "0", account_ids: [bob.accountId]});
    canBobAddKeys = await keypom.view('can_user_add_keys', {account_id: bob.accountId, drop_id: "0"});
    t.is(canBobAddKeys, false);

    // Should pass since allowlist is now empty and anyone should be able to add keys
    await owner.call(keypom, 'remove_from_sale_allowlist', {drop_id: "0", account_ids: [ali.accountId]});
    canBobAddKeys = await keypom.view('can_user_add_keys', {account_id: bob.accountId, drop_id: "0"});
    t.is(canBobAddKeys, true);

    // Bob is added to blocklist so he should not be able to add keys
    await owner.call(keypom, 'add_to_sale_blocklist', {drop_id: "0", account_ids: [bob.accountId]});
    canBobAddKeys = await keypom.view('can_user_add_keys', {account_id: bob.accountId, drop_id: "0"});
    t.is(canBobAddKeys, false);

    // Bob was removed from the blocklist so he should be able to add keys
    await owner.call(keypom, 'remove_from_sale_blocklist', {drop_id: "0", account_ids: [bob.accountId]});
    canBobAddKeys = await keypom.view('can_user_add_keys', {account_id: bob.accountId, drop_id: "0"});
    t.is(canBobAddKeys, true);

    await owner.call(keypom, 'withdraw_from_balance', {});

    // Should pass since anyone can add keys
    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[0]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});

    //owner's Keypom wallet balance should be 0 since the sale was free
    let userBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.is(userBal, "0");

    //owner should have 0 NEAR since the sale was free
    let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('ownerBal Before: ', ownerBal.toString())
    t.is(ownerBal.toString(), "0");

    await owner.call(keypom, 'update_sale', {drop_id: "0", price_per_key: NEAR.parse("1").toString()}, {attachedDeposit: NEAR.parse("10").toString()});
    await owner.call(keypom, 'withdraw_from_balance', {});

    // Should pass since anyone can add keys
    await ali.call(keypom, 'add_keys', {
        public_keys: [publicKeys[1]], 
        drop_id: "0"
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10").toString()});

    //owner should have 0 NEAR since the sale was free
    ownerBal = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('ownerBal Before: ', ownerBal.toString())
    t.is(ownerBal.toString(), NEAR.parse("1").toString());

    //should have 2 keys added
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 2);
});