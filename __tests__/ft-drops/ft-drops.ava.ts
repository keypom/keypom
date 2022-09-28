import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { generateKeyPairs, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { oneGtNear, sendFTs, totalSupply } from "./utils/ft-utils";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    keypomInitialBalance: NEAR;
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
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom });
    await ftContract.call(ftContract, 'new_default_meta', { owner_id: ftContract, total_supply: totalSupply.toString() });
    
    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const minter = await root.createSubAccount('minter');

    // Add 10k $NEAR to owner's account
    await owner.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

    // Mint the NFT
    await ftContract.call(ftContract, 'storage_deposit', { account_id: minter.accountId }, { attachedDeposit: NEAR.parse("1").toString() });
    await ftContract.call(ftContract, 'storage_deposit', { account_id: keypom.accountId }, { attachedDeposit: NEAR.parse("1").toString() });
    await ftContract.call(ftContract, 'ft_transfer', { receiver_id: minter.accountId, amount: (oneGtNear * BigInt(1000)).toString() }, { attachedDeposit: "1" });

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, owner, ali, minter, ftContract };
    t.context.keypomInitialBalance = keypomBalance.available;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Claim Multi FT Drop And Ensure Keypom Balance Increases', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    let {keys, publicKeys} = await generateKeyPairs(6);
    let ft_data = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }
    let config = {
        uses_per_key: 2,
    }

    // Creating the FT drop with 5 keys each with 2 uses per key
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys.slice(0, 5), 
        deposit_per_use: NEAR.parse("1").toString(),
        ft_data,
        config
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs
    await owner.call(keypom, 'withdraw_from_balance', {});
    // Should kickback and refund minter's balance because this isn't enough gtNEAR for 1 registered use
    await sendFTs(minter, (oneGtNear/BigInt(2)).toString(), keypom, ftContract, "0");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: 0
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
        drop_id: 0
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
        drop_id: 0
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 0);

    await owner.call(keypom, 'delete_keys', {drop_id: 0})
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
    const keypomInitialBalance = t.context.keypomInitialBalance;

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    let {keys, publicKeys} = await generateKeyPairs(2);
    let ft_data = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }

    let config = {
        uses_per_key: 10,
    }

    // Creating the FT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        ft_data,
        config
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs
    await owner.call(keypom, 'withdraw_from_balance', {});

    // Send 5 FTs registers to the contract
    await sendFTs(minter, (oneGtNear * BigInt(10)).toString(), keypom, ftContract, "0");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: 0
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
    await owner.call(keypom, 'add_keys', {drop_id: 0, public_keys: [publicKeys[0]]}, {gas: LARGE_GAS});
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
        drop_id: 0
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 0);

    await owner.call(keypom, 'delete_keys', {drop_id: 0})
    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal !== "0");
    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal === "0");

    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
});