import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { assertBalanceChange, CONTRACT_METADATA, generateKeyPairs, getDropSupplyForOwner, getKeyInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, JsonKeyInfo } from "../utils/types";

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

    // Deploy all 3 contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init empty/default linkdrop contract
    await root.call(root, 'new', {});
    //init new keypom contract and setting keypom as the owner. 
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    
    //get current keypom (contract) balances 
    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    // Test users, ali.test.near
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    
    // Custom-root.near, deploy contracts to it and init new linkdrop
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
    t.context.keypomInitialBalance = keypomBalance.available;
    t.context.keypomInitialStateStaked = keypomBalance.stateStaked;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Testing Delete On Empty Config', async t => {
    //initializing accounts, adding a balance of 10N to owner
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 2 key pairs, and then create a new config specifying that the drop is deleted when empty
    let {keys, publicKeys} = await generateKeyPairs(2);
    let config: DropConfig = {
        usage: {
            auto_delete_drop: true
        }
    }

    // Creating the drop that should be deleted when empty
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Creating the drop that should NOT be deleted when empty by passing in NO config
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[1]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config: null,
    },{gas: LARGE_GAS});

    // Set ali's NEAR account balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })
    
    //adding 2 full access keys
    for(var i = 0; i < 2; i++) {
        //set access key to be used for following transactions
        await keypom.setKey(keys[i]);
        //give full access to the key above; used to circumvent allowance bug in sandbox (should be fixed now so this can be deleted)
        await keypom.updateAccessKey(
            publicKeys[i],  // public key
            {
                nonce: 0,
                permission: 'FullAccess'
            }
        )
        //claim wallet gas to ali's account. wallet gas is 1NEAR, looping through twice should give him 2NEAR
        await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
        //after this, the access key should be deleted as it has been used. This means adding the new one is fresh. 
    }

    //ali's balance should be 2N after claiming wallet gas (constant) twice
    let aliBal = await ali.availableBalance();
    console.log('aliBal: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());
    
    //gets number of drops for given account owner, this should now be one as the first drop (drop_id = 0) should have been deleted as it was empty
    const dropSupplyForOwner = await getDropSupplyForOwner(keypom, owner.accountId);
    console.log('dropSupplyForOwner: ', dropSupplyForOwner)
    t.is(dropSupplyForOwner, 1);
    
    //drop_id 1 should still exist despite using all its keys as the delete_on_empty defaults to false and
    //drop_id 1 (second drop created) was not given explicit delete_on_empty
    const keySupplyForDrop = await getKeySupplyForDrop(keypom, "1")
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 0);
});

test('Testing Start Timestamp', async t => {
    //add 10NEAR to owner
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate key pair and get starting timestamp
    //set the starting timestamp to be 30s from now, so that you can only claim after 30s
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config: DropConfig = {
        time: {
            start: (Date.now() * 1000000) + ONE_SECOND_NS * 30,
        }
    }

    //create drop with custom starting timestamp
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Set ali's NEAR account balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

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
    // THIS SHOULD FAIL BECAUSE THE START TIMESTAMP HASN'T BEEN REACHED
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //get ali's initial balance (set to 0 earlier)
    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());
    
    //verify that there is still only one key since the claim attempt above failed
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);

    // Wait 30 seconds and call the claim method again
    await new Promise(r => setTimeout(r, 30000));
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    
    //now that the key has been used, the total key supply should now be 0
    keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 0);

    //ensure that, now that the claim has gone through, that ali's NEAR wallet balance is now 1NEAR
    aliBal = await ali.availableBalance();
    console.log('aliBal After: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());
});

test('Testing Throttle Timestamp', async t => {
    //create account and add balance of 10NEAR to owner
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair and create drop config to have 2 uses/key and a throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config: DropConfig = {
        uses_per_key: 2,
        time: {
            throttle: ONE_SECOND_NS * 30,
        }
    }

    //create a drop with the created config
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Set ali's NEAR wallet balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    //set first key as the key to be used to sign txns
    await keypom.setKey(keys[0]);
    //give this key a full access key
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // TWO CONSECUTIVE CLAIMS SHOULD FAIL BECAUSE THE THROTTLE TIMESTAMP HASN'T BEEN REACHED
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //ali should have claimed 1NEAR as the first transaction went through. Checking this here.
    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    //should still have 1 key as the single key that was added to the drop has two uses
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);
    
    //since the key was used once, it should still have one more use on it
    let keyInformation: JsonKeyInfo = await getKeyInformation(keypom, publicKeys[0]);
    console.log('keyInformation: ', keyInformation)
    t.is(keyInformation.remaining_uses, 1);

    // Wait 30 seconds and call the claim method again
    await new Promise(r => setTimeout(r, 30000));
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //now that all key uses were used up, the key should be deleted
    keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 0);

    //ali's balance, now that he's claimed twice, should be 2NEAR
    aliBal = await ali.availableBalance();
    console.log('aliBal After: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());
});

test('Testing On Claim Refund Deposit', async t => {
    //add 10NEAR to contract owner
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    //adding 10NEAR to owner's keypom debit balance
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 2 keypairs and a config with on claim refund deposit = true
    let {keys, publicKeys} = await generateKeyPairs(2);
    let config: DropConfig = {
        usage: {
            refund_deposit: true,
        }
    }

    // Creating the drop that should be deleted, this should cause owner's keypom wallet balance to be 9NEAR as they had to front the cost of 1 key at 1NEAR
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Withdrawing all owner's balance to their NEAR wallet, keypom wallet balance is now 0NEAR
    await owner.call(keypom, 'withdraw_from_balance', {});
    
    //set key to be used to sign txns
    await keypom.setKey(keys[0]);
    //give that key full access
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

    // When CLAIM is called, refund the `deposit_per_use` to the owner's account directly.
    //refund_on_claim is used when owner only wants CAAC to be used. 
    //If claim is used, the key is deleted and the deposit for said key is sent back to owner
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //confirm that ali's NEAR wallet balance is still 0 as he does not receive the funds for calling claim instead of CAAC
    let aliBal = await ali.availableBalance();
    console.log('aliBal: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    //owner's Keypom wallet balance should be at least 1NEAR as thats what should have been refunded when claim was called
    let userBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.assert(NEAR.parse("1").gte(NEAR.from(userBal)))
    
    //key was used but drop does not delete as it was not set to true in the config
    const dropSupplyForOwner = await getDropSupplyForOwner(keypom, owner.accountId);
    console.log('dropSupplyForOwner: ', dropSupplyForOwner)
    t.is(dropSupplyForOwner, 1);
    
    //the drop should no longer have any keys however, as claim was called
    const keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 0);
});

test('Testing Custom Drop Root', async t => {
    //create accounts and add 10NEAR to owner's keypom balance
    const { keypom, owner, ali, customRoot } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 2 set of keypairs and set a custom root ID, one for no-config and one for config
    let {keys: keysNoConfig, publicKeys: publicKeysNoConfig} = await generateKeyPairs(1);
    let {keys: keysConfig, publicKeys: publicKeysConfig} = await generateKeyPairs(2);
    //create config that customizes root id
    let config: DropConfig  = {
    root_account_id: customRoot.accountId,
    }
    //customRoot.accountId = customRoot.test.near

    // Creating the drop that points to the custom root using publicKeysConfig
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeysConfig, 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});
    
    // Creating the regular drop using publicKeysNoConfig
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeysNoConfig, 
        deposit_per_use: NEAR.parse("1").toString(),
        config: null,
    },{gas: LARGE_GAS});

    //generate another keypair to use in create account and claim
    let { keys: keys2, publicKeys: pks2 } = await generateKeyPairs(1);
    //foo.customRoot.test.near
    let newAccountCorrect = await keypom.getAccount(`foo.${customRoot.accountId}`);
    //foo.customRoot.test.near.test.near --> this cannot be created from a customRoot.accountId.test.near
    let newAccountIncorrect = await keypom.getAccount(`foo.${customRoot.accountId}.test.near`);
    console.log(`foo.${customRoot.accountId}.test.near: `)
    
    //non-config key set as the key being used
    await keypom.setKey(keysNoConfig[0]);
    await keypom.updateAccessKey(
        publicKeysNoConfig[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // SHOULD NOT WORK as you are using a nonConfig key & drop to deposit to a new account with a custom root
    await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `foo.${customRoot.accountId}`, new_public_key : pks2[0]}, {gas: WALLET_GAS});
    let doesExist = await newAccountCorrect.exists();
    console.log('doesExist: ', doesExist)
    t.is(doesExist, false);

    //set config key as the key being used
        await keypom.setKey(keysConfig[0]);
    await keypom.updateAccessKey(
        publicKeysConfig[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    //drain owner balance 
    await owner.call(keypom, 'withdraw_from_balance', {});
    let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    t.is(ownerBal, "0");

    ////incorrect account ID should not work
    await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `foo.${customRoot.accountId}.test.near`, new_public_key : pks2[0]}, {gas: WALLET_GAS});
    doesExist = await newAccountIncorrect.exists();
    console.log('doesExist: ', doesExist)
    t.is(doesExist, false);

    //the failed CAAC should cause a refund per deposit to be refunded to owner
    //this is since the CAAC fails on linkdrop XCC and NOT keypom internal checks
    ownerBal = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('ownerBal: ', ownerBal)
    t.assert(NEAR.from(ownerBal).gte(NEAR.parse("1")));

    //reset owner balance again
    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner.accountId});
    t.is(ownerBal, "0");

    //set second config key to be used as first one would have been deleted
    await keypom.setKey(keysConfig[1]);
    await keypom.updateAccessKey(
        publicKeysConfig[1],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )

    //this second custom root create account and claim should go through as the config key is being used
    await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `foo.${customRoot.accountId}`, new_public_key : pks2[0]}, {gas: WALLET_GAS});
    doesExist = await newAccountCorrect.exists();
    console.log('doesExist: ', doesExist)
    t.is(doesExist, true);

    //owner balance should be less than 1NEAR as they don't get refunded anymore. 
    ownerBal = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('ownerBal: ', ownerBal)
    t.assert(NEAR.from(ownerBal).lte(NEAR.parse("1")));
    });

test('Testing Auto Withdraw', async t => {
    //creating accounts, adding 100NEAR to owner's keypom wallet balance
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});

    //generate 4 keypairs
    let {keys, publicKeys} = await generateKeyPairs(4);
    //first config with 1 use/key
    let config1: DropConfig = {
        uses_per_key: 1,
    }
    //second config with auto_withdraw on, 1 use per key, and drop delete on empty enabled
    let config2: DropConfig = {
        uses_per_key: 1,
        usage: {
            auto_withdraw: true,
            auto_delete_drop: true
        }
    }
    //create drop with publickeys[0] and first config with no auto-withdraw and delete-on-empty
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config: config1,
    },{gas: LARGE_GAS});

    //second drop with publickeys[1] and second config with autowithdraw and delete-on-empty
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[1]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config: config2,
    },{gas: LARGE_GAS});
    //owner has 8NEAR in their keypom wallet from here onwards

    // Loop through 2 times and claim the keys [0, 1]
    for (let i = 0; i < 2; i++) {
        await keypom.setKey(keys[i]);
        await keypom.updateAccessKey(
            publicKeys[i],  // public key
            {
                nonce: 0,
                permission: 'FullAccess'
            }
        )
        //1st run --> 1/1key uses used on keys[0], key should be deleted but drop still exists
        //2nd run --> 1/1 key uses used on keys[1], key AND drop should be deleted as per config2
        await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
        //ali should have received 2NEAR here

    }

    // Get the owner's Keypom balance and make sure it hasn't been auto withdrawn SINCE NOT ALL OWNER'S DROP HASNT BEEN DELETED
    let userBal: String = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.assert(userBal > "0");
    
    // Delete the first drop manually, second drop was auto-deleted
    await owner.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});
    
    //get number of active drops for owner, should be 0 after first drop manual delete and second drop auto delete
    let dropSupplyForOwner = await getDropSupplyForOwner(keypom, owner.accountId);
    console.log('dropSupplyForOwner: ', dropSupplyForOwner)
    t.is(dropSupplyForOwner, 0);

    //create another drop using config2; auto-delete on, 1NEAR deposited per use
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[2]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config: config2,
    },{gas: LARGE_GAS});

    //add 1000NEAR to owner's keypom balance
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("1000").toString()});
    let shouldBeRefunded: string = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('shouldBeRefunded: ', userBal)
    t.assert(NEAR.parse("1000").lte(NEAR.from(shouldBeRefunded)));
    
    //owner's starting NEAR wallet balance
    let b1 = await owner.availableBalance();
    console.log('b1: ', b1.toString())

    //set keys[2] as the key being used
    await keypom.setKey(keys[2]);
    await keypom.updateAccessKey(
        publicKeys[2],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )

    //claim to Ali's account
    //once they key is used (and is the only and last key on this drop) and delted, the remaining balance in owner's Keypom wallet is refunded back to their NEAR wallet. 
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //b1 = 1000NEAR. b2 should be 0NEAR as the drop was deleted. This means the balance change should be 10000NEAR, or owner's starting balance
    let b2 = await owner.availableBalance();
    console.log('b2: ', b2.toString())
    t.assert(assertBalanceChange(b1, b2, NEAR.from(shouldBeRefunded), 0.01), "balance didn't decrement properly with 1% precision");

    //make sure that the owner's Keypom wallet balance is 0
    userBal = await keypom.view('get_user_balance', {account_id: owner.accountId});
    console.log('userBal: ', userBal)
    t.is(userBal, "0");

    //get number of drops for owner remaining, assert that it is 0
    dropSupplyForOwner = await getDropSupplyForOwner(keypom, owner.accountId);
    console.log('dropSupplyForOwner: ', dropSupplyForOwner)
    t.is(dropSupplyForOwner, 0);
});

test('Testing Custom Drop ID', async t => {
    //adding 1000NEAR to owner balance
    const { keypom, owner, ali, customRoot } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("1000").toString()});

    // Creating the drop has custom drop id of 1
    //this will fail, custom drop ID's < a billion WILL FAIL. This is to prevent interference with general incrementing nonce drop IDs.
    try {
        await owner.call(keypom, 'create_drop', {
            deposit_per_use: NEAR.parse("1").toString(),
            drop_id: "1"
        },{gas: LARGE_GAS});
    } catch(e) {}

    //get next drop ID and drop supply per owner.
    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
    });
    //since the last create_drop failed, the nextDropID should still be 0 and there should still be 0 drops for that owner
    console.log('viewFunctions.nextDropId: ', viewFunctions.nextDropId)
    console.log('viewFunctions.dropSupplyForOwner: ', viewFunctions.dropSupplyForOwner)
    t.is(viewFunctions.nextDropId, 0);
    t.is(viewFunctions.dropSupplyForOwner, 0);

    ////create a normal drop, no custom dropID. this means the dropID will be 0
    await owner.call(keypom, 'create_drop', {
         
        deposit_per_use: NEAR.parse("1").toString()
    },{gas: LARGE_GAS});

    //now that the first drop was created properly, the next dropID should be 1 and there shoudl be 1 drop for owner
    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
    });
    console.log('viewFunctions.nextDropId: ', viewFunctions.nextDropId)
    console.log('viewFunctions.dropSupplyForOwner: ', viewFunctions.dropSupplyForOwner)
    t.is(viewFunctions.nextDropId, 1);
    t.is(viewFunctions.dropSupplyForOwner, 1);

    //this should succeed as dropID > a billion
    await owner.call(keypom, 'create_drop', {
         
        deposit_per_use: NEAR.parse("1").toString(),
        drop_id: "2000000000"
    },{gas: LARGE_GAS});

    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
    });
    //nextDropID does not increment as a custom dropID was used
    console.log('viewFunctions.nextDropId: ', viewFunctions.nextDropId)
    console.log('viewFunctions.dropSupplyForOwner: ', viewFunctions.dropSupplyForOwner)
    t.is(viewFunctions.nextDropId, 1);
    t.is(viewFunctions.dropSupplyForOwner, 2);

    //adding another non-custom-dropID drop should go through
    await owner.call(keypom, 'create_drop', {
         
        deposit_per_use: NEAR.parse("1").toString()
    },{gas: LARGE_GAS});

    //both nextDropID and drop supply should increment here
    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
    });
    console.log('viewFunctions.nextDropId: ', viewFunctions.nextDropId)
    console.log('viewFunctions.dropSupplyForOwner: ', viewFunctions.dropSupplyForOwner)
    t.is(viewFunctions.nextDropId, 2);
    t.is(viewFunctions.dropSupplyForOwner, 3);

    //adding a duplicate dropID should not work
    try {
        await owner.call(keypom, 'create_drop', {
             
            deposit_per_use: NEAR.parse("1").toString(),
            drop_id: "2000000000"
        },{gas: LARGE_GAS});
    } catch(e) {}

    //nextDropID does not increment, neither does dropSupply
    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
    });
    console.log('viewFunctions.nextDropId: ', viewFunctions.nextDropId)
    console.log('viewFunctions.dropSupplyForOwner: ', viewFunctions.dropSupplyForOwner)
    t.is(viewFunctions.nextDropId, 2);
    t.is(viewFunctions.dropSupplyForOwner, 3);
});

test('Testing Valid Config', async t => {
    //adding 100NEAR to owner balance
    const { keypom, owner, ali } = t.context.accounts;
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});

    //zero uses per key cannot be used.
    let config: DropConfig = {
        uses_per_key: 0,
    }
    try {
        await owner.call(keypom, 'create_drop', {
             
            deposit_per_use: NEAR.parse("1").toString(),
            config,
        },{gas: LARGE_GAS});
    } catch(e) {}

    //both the start_timestamp and end timestamps are greater than the current block
    let config1: DropConfig = {
        time: {
            start: 500,
        }
    }
    try {
        await owner.call(keypom, 'create_drop', {
             
            deposit_per_use: NEAR.parse("1").toString(),
            config: config1,
        },{gas: LARGE_GAS});
    } catch(e) {}

    //end timestamp 0 not possible
    let config2: DropConfig = {
        time: {
            end: 0,
        }
    }
    try {
        await owner.call(keypom, 'create_drop', {
             
            deposit_per_use: NEAR.parse("1").toString(),
            config: config2,
        },{gas: LARGE_GAS});
    } catch(e) {}

    //end timestamp occurs before start, invalid
    let config3: DropConfig = {
        time: {
            start: (Date.now() * 1000000) + ONE_SECOND_NS * 30,
            end: (Date.now() * 1000000) + ONE_SECOND_NS * 10,
        }
    }

    try {
        await owner.call(keypom, 'create_drop', {
             
            deposit_per_use: NEAR.parse("1").toString(),
            config: config3,
        },{gas: LARGE_GAS});
    } catch(e) {}

    //none of these configs should work, hence there are no drops for that owner and nextDropID is still 0
    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
    });
    console.log('viewFunctions.dropSupplyForOwner: ', viewFunctions.dropSupplyForOwner)
    t.is(viewFunctions.dropSupplyForOwner, 0);
    t.is(viewFunctions.nextDropId, 0);
});
test('Testing End Timestamp', async t => {
    //add 10 NEAR to balance
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 2 keypairs and make a config where timestamp is 10s from now
    let {keys, publicKeys} = await generateKeyPairs(2);
    let config: DropConfig = {
        time: {
            end: (Date.now() * 1000000) + ONE_SECOND_NS * 10,
        }
    }

    //use config to create drop
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys, 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    //should have 2 keys in this specific drop
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 2);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    //set keys[0] to be used
    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // THIS SHOULD PASS as its before the end timestamp
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //should now only have 1 key in the key supply for this drop
    keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);

    // Wait 15 seconds
    await new Promise(r => setTimeout(r, 15000));

    // THIS SHOULD FAIL as the end timestamp is violated
    try {
        await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    } catch(e) {}
    
    //claim above failed, key supply stays at 1
    //note since claim failed for INTERNAL reasons, the key is NOT deleted.
    //all internal checks are performed before the key is deleted and the XCC is initiated
    keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);
});

test('Testing End Timestamp Key Drainage', async t => {
    //add 10NEAR to owner wallet
    const { keypom, owner, ali } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 keypair, create config with end timestamp 5s from now
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config: DropConfig = {
        time: {
            end: (Date.now() * 1000000) + ONE_SECOND_NS * 5,
        }
    }

    //create drop with created config
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys, 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    //key supply in this drop should now be one
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);

    // Wait 5 seconds for start timestamp to pass
    await new Promise(r => setTimeout(r, 5000));

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })
    let alib1 = await ali.availableBalance();

    //use key[0]
    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
    )

    // Loop 50 times and try to claim
    for (let i = 0; i < 50; i++) {
        try {
            //will fail as end_timestamp was violated
            await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
        } catch(e) {}
    }

    //since the key was never claimed, there is one key left for the drop, the same one originally initialized
    keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 1);

    //check if ali's balance has risen, it should not since the claims all failed
    let alib2 = await ali.availableBalance();
    t.assert(assertBalanceChange(alib1, alib2, new NEAR(0), 0.01), "balance didn't increment properly with 1% precision");
    
});

test('Testing Claim Interval', async t => {
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //create 1 keypair, config with 5uses/key, a start timestamp of 5s from now, and a 10s claim interval
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config: DropConfig = {
        uses_per_key: 5,
        time: {
            start: (Date.now() * 1000000) + ONE_SECOND_NS * 5,
            interval: ONE_SECOND_NS * 10,
        }
    }

    //create drop
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    //key supply in this drop should now be one
    let numKeysDrop1 = await getKeySupplyForDrop(keypom, "0");
    console.log('getKeySupplyForDrop: ', numKeysDrop1)
    t.is(numKeysDrop1, 1);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    //use keys[0]
    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // THIS SHOULD FAIL BECAUSE THE INTERVAL HASN'T BEEN REACHED
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //confirm that the claim did not work
    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    //wait 5 seconds to reach start timestamp [NOTE THIS WAS PASSING WITHOUT THIS I THINK...]
    await new Promise(r => setTimeout(r, 5000));

    // Wait 50 seconds
    //5 uses in the key and 10s/use means after 50s the keys can be claimed all at once
    await new Promise(r => setTimeout(r, 50000));

    // Loop through and claim all 5 times
    for (let i = 0; i < 5; i++) {
        await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    }

    //all claims should have gone through, expect key to be deleted
    let keySupplyForDrop = await getKeySupplyForDrop(keypom, "0");
    console.log('keySupplyForDrop: ', keySupplyForDrop)
    t.is(keySupplyForDrop, 0);

    //aliBal should now be 5 after claiming 5 times at 1NEAR per key use
    aliBal = await ali.availableBalance();
    console.log('aliBal After: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("5").toString());
});

test('Testing All Time Based Configs Together', async t => {
    //create owner with 10NEAR balance
    const { keypom, owner, ali } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 1 key pair and a config with 5uses per key, a start timestamp of 30s from now, a claim interval of 10s and throttle timestamp of 30s
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config: DropConfig = {
        uses_per_key: 5,
        time: {
            start: (Date.now() * 1000000) + ONE_SECOND_NS * 30,
            interval: ONE_SECOND_NS * 10,
            throttle: ONE_SECOND_NS * 30
        }
    }

    //create drop using above defined keys and config
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    //set keys[0] to be claimed
    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        publicKeys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // THIS SHOULD FAIL BECAUSE THE INTERVAL HASN'T BEEN REACHED
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    //make sure above claim failed
    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // Wait 60 seconds to pass total claim interval AND start timestamp
    await new Promise(r => setTimeout(r, 60000));


    // Loop through and claim all 5 times But ONLY ONE should be claimed due to throttle timestamp
    for (let i = 0; i < 5; i++) {
        await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    }

    //ali should only have 1NEAR after those claims as only the first one through
    aliBal = await ali.availableBalance();
    console.log('aliBal After: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // Wait 30 seconds for throttle timestamp to pass
    await new Promise(r => setTimeout(r, 30000));

    // Loop through and claim remaining 4 times with a pause that matches throttle timestamp. This should claim the remaning used on the key
    for (let i = 0; i < 4; i++) {
        await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
        // Wait 30 seconds
        await new Promise(r => setTimeout(r, 30000));
    }

    //ali shoud now howvae 5NEAR after the key was claimed 5 times.
    aliBal = await ali.availableBalance();
    console.log('aliBal After: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("5").toString());
});