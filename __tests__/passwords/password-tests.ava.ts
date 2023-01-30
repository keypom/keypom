import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { oneGtNear, totalSupply } from "../ft-drops/utils/ft-utils";
import { assertBalanceChange, CONTRACT_METADATA, generateKeyPairs, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, FTData } from "../utils/types";
import { generateGlobalPasswords, generateLocalPasswords, hash } from "./utils/pwUtils";

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

    // Add 10k $NEAR to owner's account
    await owner.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

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

test('Multi-use keys with local passwords', async t => {
    // Comment this if you want to see console logs
    //console.log = function() {}
    
    //generate 1 keypair, 3 uses per key config created
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config: DropConfig = {
        uses_per_key: 3,
    }

    //set base pw, create new keysWithPws data structure
    let basePassword = 'mypassword1';
    let keysWithPws: { [key: string]: number[] } = {};
    //set publicKeys[0] to have pws for use 1, 2, 3
    keysWithPws[publicKeys[0]] = [1, 2, 3];

    //generates an array of hash(hash(basePassword + publicKeys[i])) --> all PWs for all key use
    let passwords = generateLocalPasswords(publicKeys, keysWithPws, basePassword);
    console.log('passwords: ', passwords)

    //add 10 $NEAR to owner balance
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //create drop using public key, indicating 3 key uses
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys, 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
        passwords_per_use: passwords
    },{gas: LARGE_GAS});

    //use keys[0]
    await keypom.setKey(keys[0]);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    // THIS SHOULD FAIL BECAUSE NO PASSWORD PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});

    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH (ACTUAL PW) IS PASSED IN [it's going to be hashed AGAIN on the contract]
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePassword + publicKeys[0] + '1'), true)}, {gas: WALLET_GAS});

    //check if aliBal is empty, should be the case as none of his claims went through
    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    // THIS SHOULD FAIL as he is reusing the same pw
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    //make sure Ali has only been able to claim ONCE
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '2')}, {gas: WALLET_GAS});
    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '3')}, {gas: WALLET_GAS});

    //Ali has now claimed 3 times and should have 3 $NEAR
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("3").toString());
});

test('2 keys have local (first with all use pw second with only 1 use pw), 1 has global, 1 has none. Global and local have different base pws', async t => {
    // Comment this if you want to see console logs
    //console.log = function() {}

    //LOCAL KEYS ARE PER USE, GLOBAL ARE PER KEY!
    
    //create drop config with 2 uses per key, set base pw for global (pw per key) and local (pw per use)
    //create 4 keypairs for use later
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
    let {keys, publicKeys} = await generateKeyPairs(4);
    let config: DropConfig = {
        uses_per_key: 2,
    }
    let basePasswordLocal = 'mypassword1';
    let basePasswordGlobal = 'mypassword2';

    //LOCAL KEYS INDICATING DIFF PW PER USE
    //first two keypairs have a pw. publicKeys[0] has pw at [1,2]. the other has pw at [1]
    let keysWithPws: { [key: string]: number[] } = {};
    keysWithPws[publicKeys[0]] = [1, 2];
    keysWithPws[publicKeys[1]] = [1];

    //generate local and global pw's
    let localPasswords = generateLocalPasswords(publicKeys, keysWithPws, basePasswordLocal);
    console.log('localPasswords: ', localPasswords)
    let globalPasswords = generateGlobalPasswords(publicKeys, [publicKeys[2]], basePasswordGlobal);
    console.log('globalPasswords: ', globalPasswords)

    //add 100 $NEAR to owner
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});

    //create drop with pw per key and pw per use
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys, 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
        passwords_per_use: localPasswords,
        passwords_per_key: globalPasswords
    },{gas: LARGE_GAS});

    /*
    *   LOCAL KEY #1
    */
    await keypom.setKey(keys[0]);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    // THIS SHOULD FAIL BECAUSE NO PASSWORD PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePasswordLocal + publicKeys[0] + '1'), true)}, {gas: WALLET_GAS});

    //verify the two claims above have failed
    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());
    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL, these keys with local pw can only be used once, then that key is deleted similar to claim w/o pw
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    //ensure ali only gained 1 $NEAR from 1 successful claim
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '2')}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    //alibal should now be 2 $NEAR
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());


    /*
    *   LOCAL KEY #2 
    */
    await keypom.setKey(keys[1]);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    // THIS SHOULD FAIL BECAUSE NO PASSWORD PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePasswordLocal + publicKeys[1] + '1'), true)}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[1] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD PASS SINCE THERE'S NO PASSWORD ON THE SECOND CLAIM
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});
    //no pw but passing in pw should still allow to claim

    //both claims passed, should now have 2 $NEAR balance
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());

    /*
    *   GLOBAL KEY #1 
    */
    await keypom.setKey(keys[2]);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    // THIS SHOULD FAIL BECAUSE NO PASSWORD PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL BECAUSE IT'S A GLOBAL PW NOT LOCAL, does not need + '1'
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2] + '1')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    //since global pw, both key uses should have the SAME PW.

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2])}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2])}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    //both should pass, alibal should now be 2 $NEAR
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());
});

test('Add keys after drop is created with passwords', async t => {
    //generate 5 keypairs, 2 use per key drop config
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
    let {keys, publicKeys} = await generateKeyPairs(5);
    let config: DropConfig = {
        uses_per_key: 2,
    }
    //set base for global and local pws
    let basePasswordLocal = 'mypassword1';
    let basePasswordGlobal = 'mypassword2';

    // keep track of the local pw's. First has 2 pw's, second has 1.
    let keysWithPws: { [key: string]: number[] } = {};
    keysWithPws[publicKeys[0]] = [1, 2];
    keysWithPws[publicKeys[1]] = [1];

    //generate local and global pw's 
    let localPasswords = generateLocalPasswords(publicKeys.slice(0, 4), keysWithPws, basePasswordLocal);
    console.log('localPasswords: ', localPasswords)
    let globalPasswords = generateGlobalPasswords(publicKeys.slice(0, 4), [publicKeys[2]], basePasswordGlobal);
    console.log('globalPasswords: ', globalPasswords)

    //add 100 $NEAR to owner's wallet
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});

    //create drop with no pw, and the last keypair
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[4]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    //add keys to the drop created above, specify the passwords here and add the other 4 keypairs created
    //reminder,publicKeys[0] has 2 local pws, publicsKeys[1] has 1 local pw, publicKeys[2] has 1 global and publicKeys[3] has no pw
    await owner.call(keypom, 'add_keys', {
        public_keys: publicKeys.slice(0, 4), 
        drop_id: '0',
        passwords_per_use: localPasswords,
        passwords_per_key: globalPasswords,
    },{gas: LARGE_GAS});

    /*
    *   LOCAL KEY #1 --> 2 local PWs
    */
    await keypom.setKey(keys[0]);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    // THIS SHOULD FAIL BECAUSE NO PASSWORD PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePasswordLocal + publicKeys[0] + '1'), true)}, {gas: WALLET_GAS});

    //get ali's $NEAR balance before any deposites go through
    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());
    // THIS SHOULD PASS, ali should gain 1 $NEAR
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL as this pw was already used, must pass in the next pw
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    //verify only one claim succeeded
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS as the second pw is now being used
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '2')}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());


    /*
    *   LOCAL KEY #2 - ONLY FIRST USE HAS LOCAL PW
    */
    await keypom.setKey(keys[1]);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    // THIS SHOULD FAIL BECAUSE NO PASSWORD PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePasswordLocal + publicKeys[1] + '1'), true)}, {gas: WALLET_GAS});

    //ensure both claims above failed
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[1] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD PASS SINCE THERE'S NO PASSWORD ON THE SECOND CLAIM
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    //ensure both claims succeedded
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());

    /*
    *   GLOBAL KEY #1 - 1 single global pw
    */
    await keypom.setKey(keys[2]);

    // Set ali's balance to 0 so we can check if the claim works properly
    await ali.updateAccount({
        amount: "0"
    })

    // THIS SHOULD FAIL BECAUSE NO PASSWORD PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL BECAUSE IT'S A GLOBAL PW NOT LOCAL
    //adding + '1' indicated that its local.
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2] + '1')}, {gas: WALLET_GAS});

    //ensure both cases above have failed
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS AS THE GLOBAL PW IS USED
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2])}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS AS GLOBAL PW WORKS FOR ALL CLAIMS ON THAT KEY
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2])}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());
});

test('50 FT Keys Fails in Step 2 Check Storage', async t => {
    //This test combines FT drops with pw protected ones. Setup all necessary info for FT drop
    const { keypom, owner, ali, minter, root } = t.context.accounts;
    const ftContract = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    await ftContract.call(ftContract, 'new_default_meta', { owner_id: ftContract, total_supply: totalSupply.toString() });
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});

    let numKeys = 50;

    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: owner.accountId,
        balance_per_use: oneGtNear.toString()
    }
    let basePassword = 'mypassword1';
    
    //create 50 keypairs, give them all passwords on the first use (key X has pw on use 1)
    let {keys: keys1, publicKeys: publicKeys1} = await generateKeyPairs(numKeys);
    let keysWithPws: { [key: string]: number[] } = {};
    for (var i = 0; i < publicKeys1.length; i++) {
        keysWithPws[publicKeys1[i]] = [1];
    }
    //generate passwords
    let passwords = generateLocalPasswords(publicKeys1, keysWithPws, basePassword);

    //get keypom's initial $NEAR wallet balance
    let kpBalInit1 = await keypom.balance();
    
    //don't add any $NEAR to owner's account
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("0").toString()});

    // Creating the FT drop with 50 keys and password per use for 1 use
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys1, 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        passwords_per_use: passwords
    },{gas: LARGE_GAS});

    //get rid of drop owner's Keypom balance so that they don't have the funds to pay for FT storage
    console.log('Owner removes balance from keypom')
    await owner.call(keypom, 'withdraw_from_balance', {});

    //keypom's final balance should be more than or equal to when it started; KEYPOM SHOULD NOT LOSE MONEY HERE
    let kpBalFin1 = await keypom.balance();
    t.assert(kpBalFin1.available.gte(kpBalInit1.available));


    //second set of keys, 50 keys again and giving them all first use pw's
    let {keys: keys2, publicKeys: publicKeys2} = await generateKeyPairs(numKeys);
    keysWithPws = {};
    for (var i = 0; i < publicKeys1.length; i++) {
        keysWithPws[publicKeys2[i]] = [1];
    }
    passwords = generateLocalPasswords(publicKeys2, keysWithPws, basePassword);

    //get Keypom's initial balance again
    let kpBalInit2 = await keypom.balance();
    
    // console.log("adding to balance");
    //add just enough $NEAR for creating drop and adding FTs but not the storage?
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("51.511").toString()});

    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys2, 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        passwords_per_use: passwords
    },{gas: LARGE_GAS});

    console.log('Owner removes balance from keypom')
    await owner.call(keypom, 'withdraw_from_balance', {});

    let kpBalFin2 = await keypom.balance();
    t.assert(kpBalFin2.available.gte(kpBalInit2.available));
});

test('Create 50 key drop and delete after', async t => {
    const { keypom, owner, ali, minter, root } = t.context.accounts;
    const ftContract = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);
    await ftContract.call(ftContract, 'new_default_meta', { owner_id: ftContract, total_supply: totalSupply.toString() });
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});

    let numKeys = 50;

    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: owner.accountId,
        balance_per_use: oneGtNear.toString()
    }
    let basePassword = 'mypassword1';
    
    let {keys: keys1, publicKeys: publicKeys1} = await generateKeyPairs(numKeys);
    let keysWithPws: { [key: string]: number[] } = {};
    for (var i = 0; i < publicKeys1.length; i++) {
        keysWithPws[publicKeys1[i]] = [1];
    }
    let passwords = generateLocalPasswords(publicKeys1, keysWithPws, basePassword);
    
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("55").toString()});

    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys1, 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        passwords_per_use: passwords
    },{gas: LARGE_GAS});

    console.log('Owner removes balance from keypom')
    await owner.call(keypom, 'withdraw_from_balance', {});

    let kpBalInit1 = await keypom.balance();
    console.log('keypom available INITIAL: ', kpBalInit1.available.toString())
    console.log('keypom staked INITIAL: ', kpBalInit1.staked.toString())
    console.log('keypom stateStaked INITIAL: ', kpBalInit1.stateStaked.toString())
    console.log('keypom total INITIAL: ', kpBalInit1.total.toString())

    await owner.call(keypom, 'delete_keys', {drop_id: "0"}, {gas: LARGE_GAS});

    let kpBalFin1 = await keypom.balance();
    console.log('keypom available FINAL: ', kpBalFin1.available.toString())
    console.log('keypom staked FINAL: ', kpBalFin1.staked.toString())
    console.log('keypom stateStaked FINAL: ', kpBalFin1.stateStaked.toString())
    console.log('keypom total FINAL: ', kpBalFin1.total.toString())
    t.assert(kpBalFin1.available.gte(kpBalInit1.available));
});