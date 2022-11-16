import anyTest, { TestFn } from "ava";
import { BN } from "bn.js";
import { NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { assertBalanceChange, CONTRACT_METADATA, DEFAULT_GAS, GAS_PRICE, generateKeyPairs, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, FTData } from "../utils/types";
import { ftRegistrationFee, oneGtNear, sendFTs, totalSupply } from "./utils/ft-utils";

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

    // Deploy all 3 contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const ftContract = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);

    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    await ftContract.call(ftContract, 'new_default_meta', { owner_id: ftContract, total_supply: totalSupply.toString() });
    
    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const minter = await root.createSubAccount('minter');

    // Add 10k $NEAR to owner's account
    await owner.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: "0"});

    // Mint the FTs
    await ftContract.call(ftContract, 'storage_deposit', { account_id: minter.accountId }, { attachedDeposit: NEAR.parse("1").toString() });
    await ftContract.call(ftContract, 'ft_transfer', { receiver_id: minter.accountId, amount: (oneGtNear * BigInt(1000)).toString() }, { attachedDeposit: "1" });

    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, owner, ali, minter, ftContract };
    t.context.keypomInitialBalance = keypomBalance.available;
    t.context.keypomInitialStateStaked = keypomBalance.stateStaked;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Claim Multi FT Drop And Ensure Keypom Balance Increases', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});
    let storageBal = await ftContract.view('storage_balance_of', { account_id: keypom.accountId });
    console.log('storageBal: ', storageBal)
    t.not(storageBal, null);
    
    const keypomInitialBalance = t.context.keypomInitialBalance;

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    let {keys, publicKeys} = await generateKeyPairs(6);
    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }
    let config: DropConfig = {
        uses_per_key: 2,
    }

    // Creating the FT drop with 5 keys each with 2 uses per key
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys.slice(0, 5), 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        config
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs
    await owner.call(keypom, 'withdraw_from_balance', {});
    // Should kickback and refund minter's balance because this isn't enough gtNEAR for 1 registered use
    await sendFTs(minter, (oneGtNear/BigInt(2)).toString(), keypom, ftContract, "0");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);

    let minterBal = await ftContract.view('ft_balance_of', { account_id: minter.accountId });
    console.log('minterBal: ', minterBal)
    t.is(minterBal, (oneGtNear * BigInt(1000)).toString());

    // Register all 10 uses
    await sendFTs(minter, (oneGtNear*BigInt(10)).toString(), keypom, ftContract, "0");

    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 10);

    minterBal = await ftContract.view('ft_balance_of', { account_id: minter.accountId });
    console.log('minterBal: ', minterBal)
    t.is(minterBal, (oneGtNear * BigInt(990)).toString());

    let keypomBal = await ftContract.view('ft_balance_of', { account_id: keypom.accountId });
    console.log('keypomBal: ', keypomBal)
    t.is(keypomBal, (oneGtNear * BigInt(10)).toString());

    for(let i = 0; i < 5; i++) {
        await keypom.setKey(keys[i]);
        await keypom.updateAccessKey(
            keys[i],  // public key
            {
                nonce: 0,
                permission: 'FullAccess'
            }
        )

        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : publicKeys[5]}, {gas: WALLET_GAS});
        await keypom.call(keypom, 'claim', {account_id: `${i}.test.near`}, {gas: WALLET_GAS});

        let newUserBal = await ftContract.view('ft_balance_of', { account_id: `${i}.test.near` });
        console.log(`account ID: ${i}.test.near`)
        console.log('newUserBal: ', newUserBal)
        t.is(newUserBal, (oneGtNear * BigInt(2)).toString());
    }

    keypomBal = await ftContract.view('ft_balance_of', { account_id: keypom.accountId });
    console.log('keypomBal AFTER: ', keypomBal)
    t.is(keypomBal, "0");

    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 0);

    await owner.call(keypom, 'delete_keys', {drop_id: "0"})
    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal !== "0");
    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal === "0");

    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
});

test('OverRegister FTs and add multi use key later', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});
    const keypomInitialBalance = t.context.keypomInitialBalance;

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    let {keys, publicKeys} = await generateKeyPairs(2);
    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }

    let config: DropConfig = {
        uses_per_key: 10,
    }

    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        config
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs
    await owner.call(keypom, 'withdraw_from_balance', {});

    // Send 5 FTs registers to the contract
    await sendFTs(minter, (oneGtNear * BigInt(10)).toString(), keypom, ftContract, "0");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.keysForDrop?.length, 0);
    t.is(viewFunctions.dropInformation?.registered_uses, 10);

    let minterBal = await ftContract.view('ft_balance_of', { account_id: minter.accountId });
    console.log('minterBal: ', minterBal)
    t.is(minterBal, (oneGtNear * BigInt(990)).toString());

    let keypomBal = await ftContract.view('ft_balance_of', { account_id: keypom.accountId });
    console.log('keypomBal: ', keypomBal)
    t.is(keypomBal, (oneGtNear * BigInt(10)).toString());

    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});
    await owner.call(keypom, 'add_keys', {drop_id: '0', public_keys: [publicKeys[0]]}, {gas: LARGE_GAS});
    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        keys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    for(let i = 0; i < 5; i++) {
        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : publicKeys[1]}, {gas: WALLET_GAS});
        await keypom.call(keypom, 'claim', {account_id: `${i}.test.near`}, {gas: WALLET_GAS});

        let newUserBal = await ftContract.view('ft_balance_of', { account_id: `${i}.test.near` });
        console.log(`account ID: ${i}.test.near`)
        console.log('newUserBal: ', newUserBal)
        t.is(newUserBal, (oneGtNear * BigInt(2)).toString());
    }

    keypomBal = await ftContract.view('ft_balance_of', { account_id: keypom.accountId });
    console.log('keypomBal: ', keypomBal)
    t.is(keypomBal, "0");

    minterBal = await ftContract.view('ft_balance_of', { account_id: minter.accountId });
    console.log('minterBal: ', minterBal)
    t.is(minterBal, (oneGtNear * BigInt(990)).toString());

    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 0);

    await owner.call(keypom, 'delete_keys', {drop_id: "0"})
    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal !== "0");
    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal === "0");

    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
}); 

test('Deleting Keys and Drop', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});
    const keypomInitialBalance = await (await keypom.balance()).available;

    let {keys, publicKeys} = await generateKeyPairs(6);
    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }
    
    console.log("adding to balance");
    // How much does it cost to create a drop?
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    await owner.call(keypom, 'create_drop', {   
        deposit_per_use: NEAR.parse("1").toString(),
        ft
    },{gas: LARGE_GAS});
    let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating key: ', ownerBal)
    let netCostCreatingDrop = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostCreatingDrop: ', netCostCreatingDrop.toString())

    // Measure how much $NEAR it costs to add a single key
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    await owner.call(keypom, 'add_keys', {
        public_keys: [publicKeys[0]],
        drop_id: '0'
    },{gas: LARGE_GAS});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating key: ', ownerBal)
    let netCostAddingOneKey = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostAddingOneKey: ', netCostAddingOneKey.toString())
    await owner.call(keypom, 'withdraw_from_balance', {});

    // Remove the key and ensure the owner balance goes up by the same net cost
    await owner.call(keypom, 'delete_keys', {drop_id: "0", delete_on_empty: false});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('new ownerBal after del: ', ownerBal)
    t.is(NEAR.from(ownerBal).toString(), netCostAddingOneKey.toString());

    // Delete the drop and ensure the owner balance goes up by the net cost
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'delete_keys', {drop_id: "0"});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('new ownerBal after del drop: ', ownerBal)
    t.is(NEAR.from(ownerBal).toString(), netCostCreatingDrop.toString());

    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal === "0");

    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    console.log('keypom staked FINAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked FINAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total FINAL: ', keypomBalance.total.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);

    // Creating a new drop with 1 key and checking if the net cost is equal to the cost of adding a single key + 1 drop
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]],
        deposit_per_use: NEAR.parse("1").toString(),
        ft
    },{gas: LARGE_GAS});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating key: ', ownerBal)
    let netCostCreatingDropWithOneKey = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostCreatingDropWithOneKey: ', netCostCreatingDropWithOneKey.toString())
    t.is(netCostCreatingDropWithOneKey.toString(), netCostAddingOneKey.add(netCostCreatingDrop).toString());
});

test('Refunding Assets and Deleting Multi Use Keys and Drops', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});
    const keypomInitialBalance = await (await keypom.balance()).available;
    console.log('keypomInitialBalance: ', keypomInitialBalance)

    let {keys, publicKeys} = await generateKeyPairs(2);
    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }

    let config: DropConfig = {
        uses_per_key: 10,
    }

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        config
    },{gas: LARGE_GAS});
    let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating drop with key: ', ownerBal)
    let netCost = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostCreatingDrop: ', netCost.toString())
    await owner.call(keypom, 'withdraw_from_balance', {});

    // Send 5 FTs registers to the contract
    await sendFTs(minter, (oneGtNear * BigInt(10)).toString(), keypom, ftContract, "0");

    await keypom.setKey(keys[0]);
    await keypom.updateAccessKey(
        keys[0],  // public key
        {
            nonce: 0,
            permission: 'FullAccess'
        }
    )
    // Use the key 5 out of 10 times
    for(let i = 0; i < 5; i++) {
        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : publicKeys[1]}, {gas: WALLET_GAS});

        let newUserBal = await ftContract.view('ft_balance_of', { account_id: `${i}.test.near` });
        console.log(`account ID: ${i}.test.near`)
        console.log('newUserBal: ', newUserBal)
        t.is(newUserBal, (oneGtNear * BigInt(1)).toString());
    }

    let keypomBal = await ftContract.view('ft_balance_of', { account_id: keypom.accountId });
    console.log('keypomBal: ', keypomBal)
    t.is(keypomBal, (oneGtNear * BigInt(5)).toString());

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 5);
    t.is(viewFunctions.keysForDrop?.length, 1);

    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'refund_assets', {drop_id: "0"})
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.is(ownerBal, "0");

    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 1);

    keypomBal = await ftContract.view('ft_balance_of', { account_id: keypom.accountId });
    console.log('keypomBal: ', keypomBal)
    t.is(keypomBal, "0");

    let minterBal = await ftContract.view('ft_balance_of', { account_id: minter.accountId });
    console.log('minterBal: ', minterBal)
    t.is(minterBal, (oneGtNear * BigInt(995)).toString());

    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'delete_keys', {drop_id: "0"});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal > "0");
    await owner.call(keypom, 'withdraw_from_balance', {});

    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
});

test('Paying with Attached Deposit. FT Contract Does Not Exist', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});
    const keypomInitialBalance = t.context.keypomInitialBalance;

    let {keys, publicKeys} = await generateKeyPairs(2);
    let ft: FTData = {
        contract_id: "foobar.test.near",
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }

    let config: DropConfig = {
        uses_per_key: 10,
    }

    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        config
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("15").toString()});

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId
    });
    console.log('viewFunctions.dropsForOwner: ', viewFunctions.dropsForOwner)
    t.is(viewFunctions.dropsForOwner?.length, 0);

    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.is(ownerBal, "0");
}); 

test('Paying with Attached Deposit. Not enough deposit to cover callback registration fee', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});
    const keypomInitialBalance = t.context.keypomInitialBalance;

    let {keys, publicKeys} = await generateKeyPairs(2);
    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }

    let config: DropConfig = {
        uses_per_key: 10,
    }

    
    let b1 = await owner.availableBalance();    
    console.log('b1: ', b1.toString())
    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        config
    },{gas: LARGE_GAS, attachedDeposit: NEAR.parse("10.202").toString()});
    // Wait 5 seconds
    await new Promise(r => setTimeout(r, 5000));

    let b2 = await owner.availableBalance();
    console.log('b2: ', b2.toString())
    // Should only go down by about 20 TGas
    t.assert(assertBalanceChange(b1, b2, NEAR.parse("0.0021"), 0.02), "balance didn't decrement properly with 2% precision");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId
    });
    console.log('viewFunctions.dropsForOwner: ', viewFunctions.dropsForOwner)
    t.is(viewFunctions.dropsForOwner?.length, 0);

    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.is(ownerBal, "0");
}); 

test('Paying with User Balance. FT Contract Does Not Exist', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});
    const keypomInitialBalance = t.context.keypomInitialBalance;

    let {keys, publicKeys} = await generateKeyPairs(2);
    let ft: FTData = {
        contract_id: "foobar.test.near",
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }

    let config: DropConfig = {
        uses_per_key: 10,
    }

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("15").toString()});

    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        config
    },{gas: LARGE_GAS});

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId
    });
    console.log('viewFunctions.dropsForOwner: ', viewFunctions.dropsForOwner)
    t.is(viewFunctions.dropsForOwner?.length, 0);

    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.is(ownerBal, NEAR.parse("15").toString());
}); 

// Add a test checking if you create 1 drop then another, the first one will cost more since FT contract needs to be registered
test('Automatically Register Keypom Contract', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    let {keys, publicKeys} = await generateKeyPairs(2);
    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        ft
    },{gas: LARGE_GAS});
    let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating drop with key: ', ownerBal)
    let netCostDrop1 = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostDrop1: ', netCostDrop1.toString())
    await owner.call(keypom, 'withdraw_from_balance', {});

    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[1]], 
        deposit_per_use: NEAR.parse("1").toString(),
        ft
    },{gas: LARGE_GAS});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating second drop with key: ', ownerBal)
    let netCostDrop2 = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostDrop2: ', netCostDrop2.toString())
    await owner.call(keypom, 'withdraw_from_balance', {});
    t.assert(netCostDrop1.gt(netCostDrop2), "net cost of second drop should be less than first drop");

    let storageBal = await ftContract.view('storage_balance_of', { account_id: keypom.accountId });
    console.log('storageBal: ', storageBal)
    t.not(storageBal, null);

    await owner.call(keypom, 'delete_keys', {drop_id: "0"});
    await owner.call(keypom, 'delete_keys', {drop_id: "1"});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal > "0");
    await owner.call(keypom, 'withdraw_from_balance', {});

    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
}); 