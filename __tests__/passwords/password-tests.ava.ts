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
    
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
    let {keys, publicKeys} = await generateKeyPairs(1);
    let config: DropConfig = {
        uses_per_key: 3,
    }

    let basePassword = 'mypassword1';
    let keysWithPws: { [key: string]: number[] } = {};
    keysWithPws[publicKeys[0]] = [1, 2, 3];

    let passwords = generateLocalPasswords(publicKeys, keysWithPws, basePassword);
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

    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePassword + publicKeys[0] + '1'), true)}, {gas: WALLET_GAS});

    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    // THIS SHOULD FAIL
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '2')}, {gas: WALLET_GAS});
    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePassword + publicKeys[0] + '3')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("3").toString());
});

test('2 keys have local (first with all use pw second with only 1 use pw), 1 has global, 1 has none. Global and local have different base pws', async t => {
    // Comment this if you want to see console logs
    //console.log = function() {}
    
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
    let {keys, publicKeys} = await generateKeyPairs(4);
    let config: DropConfig = {
        uses_per_key: 2,
    }
    let basePasswordLocal = 'mypassword1';
    let basePasswordGlobal = 'mypassword2';

    let keysWithPws: { [key: string]: number[] } = {};
    keysWithPws[publicKeys[0]] = [1, 2];
    keysWithPws[publicKeys[1]] = [1];

    let localPasswords = generateLocalPasswords(publicKeys, keysWithPws, basePasswordLocal);
    console.log('localPasswords: ', localPasswords)
    let globalPasswords = generateGlobalPasswords(publicKeys, [publicKeys[2]], basePasswordGlobal);
    console.log('globalPasswords: ', globalPasswords)

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});

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
    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePasswordLocal + publicKeys[0] + '1'), true)}, {gas: WALLET_GAS});

    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());
    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '2')}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());


    /*
    *   LOCAL KEY #2 
    */
    await keypom.setKey(keys[1]);
    await keypom.updateAccessKey(
        publicKeys[1],  // public key
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
    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePasswordLocal + publicKeys[1] + '1'), true)}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[1] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD PASS SINCE THERE'S NO PASSWORD ON THE SECOND CLAIM
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());

    /*
    *   GLOBAL KEY #1 
    */
    await keypom.setKey(keys[2]);
    await keypom.updateAccessKey(
        publicKeys[2],  // public key
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
    // THIS SHOULD FAIL BECAUSE IT'S A GLOBAL PW NOT LOCAL
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2] + '1')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2])}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2])}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());
});

test('Add keys after drop is created with passwords', async t => {
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
    let {keys, publicKeys} = await generateKeyPairs(5);
    let config: DropConfig = {
        uses_per_key: 2,
    }
    let basePasswordLocal = 'mypassword1';
    let basePasswordGlobal = 'mypassword2';

    let keysWithPws: { [key: string]: number[] } = {};
    keysWithPws[publicKeys[0]] = [1, 2];
    keysWithPws[publicKeys[1]] = [1];

    let localPasswords = generateLocalPasswords(publicKeys.slice(0, 4), keysWithPws, basePasswordLocal);
    console.log('localPasswords: ', localPasswords)
    let globalPasswords = generateGlobalPasswords(publicKeys.slice(0, 4), [publicKeys[2]], basePasswordGlobal);
    console.log('globalPasswords: ', globalPasswords)

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});

    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[4]], 
        deposit_per_use: NEAR.parse("1").toString(),
        config,
    },{gas: LARGE_GAS});

    await owner.call(keypom, 'add_keys', {
        public_keys: publicKeys.slice(0, 4), 
        drop_id: '0',
        passwords_per_use: localPasswords,
        passwords_per_key: globalPasswords,
    },{gas: LARGE_GAS});

    /*
    *   LOCAL KEY #1
    */
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
    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePasswordLocal + publicKeys[0] + '1'), true)}, {gas: WALLET_GAS});

    let aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());
    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD FAIL
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '2')}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());


    /*
    *   LOCAL KEY #2 
    */
    await keypom.setKey(keys[1]);
    await keypom.updateAccessKey(
        publicKeys[1],  // public key
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
    // THIS SHOULD FAIL BECAUSE THE DOUBLE HASH IS PASSED IN
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(hash(basePasswordLocal + publicKeys[1] + '1'), true)}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[1] + '1')}, {gas: WALLET_GAS});
    // THIS SHOULD PASS SINCE THERE'S NO PASSWORD ON THE SECOND CLAIM
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordLocal + publicKeys[0] + '1')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());

    /*
    *   GLOBAL KEY #1 
    */
    await keypom.setKey(keys[2]);
    await keypom.updateAccessKey(
        publicKeys[2],  // public key
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
    // THIS SHOULD FAIL BECAUSE IT'S A GLOBAL PW NOT LOCAL
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2] + '1')}, {gas: WALLET_GAS});

    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("0").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2])}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("1").toString());

    // THIS SHOULD PASS
    await keypom.call(keypom, 'claim', {account_id: ali.accountId, password: hash(basePasswordGlobal + publicKeys[2])}, {gas: WALLET_GAS});
    aliBal = await ali.availableBalance();
    console.log('aliBal Before: ', aliBal.toString())
    t.is(aliBal.toString(), NEAR.parse("2").toString());
});

test('50 FT Keys Fails in Step 2 Check Storage', async t => {
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

    let kpBalInit1 = await keypom.balance();
    
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("0").toString()});

    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys1, 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        passwords_per_use: passwords
    },{gas: LARGE_GAS});

    console.log('Owner removes balance from keypom')
    await owner.call(keypom, 'withdraw_from_balance', {});

    let kpBalFin1 = await keypom.balance();
    t.assert(kpBalFin1.available.gte(kpBalInit1.available));


    let {keys: keys2, publicKeys: publicKeys2} = await generateKeyPairs(numKeys);
    keysWithPws = {};
    for (var i = 0; i < publicKeys1.length; i++) {
        keysWithPws[publicKeys2[i]] = [1];
    }
    passwords = generateLocalPasswords(publicKeys2, keysWithPws, basePassword);

    let kpBalInit2 = await keypom.balance();
    
    // console.log("adding to balance");
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