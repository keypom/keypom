import anyTest, { TestFn } from "ava";
import { BN } from "bn.js";
import { ExecutionStatusBasic, NEAR, NearAccount, Worker } from "near-workspaces";
import { assertBalanceChange, CONTRACT_METADATA, generateKeyPairs, GAS_PRICE, LARGE_GAS, DEFAULT_GAS, queryAllViewFunctions, WALLET_GAS, displayFailureLog } from "../utils/general";
import { DropConfig, FCData, JsonKeyInfo, JsonNFTData, JsonToken, FTData } from "../utils/types";
import { keypom_args, mintNFTs, nftMetadata, nftSeriesMetadata, sendNFTs } from "./utils/nft-utils";
import { ftRegistrationFee, oneGtNear, sendFTs, totalSupply } from "./utils/ft-utils";
import { parseNearAmount } from "near-api-js/lib/utils/format";


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
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nested-fields-nft.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const bob = await root.createSubAccount('bob');
    
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: owner.accountId });
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: ali.accountId });
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: bob.accountId });
    
    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    let nftBalance = await nftContract.balance();
    console.log('nftContract available INITIAL: ', nftBalance.available.toString())
    console.log('nftContract staked INITIAL: ', nftBalance.staked.toString())
    console.log('nftContract stateStaked INITIAL: ', nftBalance.stateStaked.toString())
    console.log('nftContract total INITIAL: ', nftBalance.total.toString())

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, nftContract, owner, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Simple Drop', async t => {
    const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

    const config: DropConfig = { 
        uses_per_key: 1,
        usage:{
            refund_deposit: true,
        }
    }

    // drop creation cost
    await ali.call(keypom, 'withdraw_from_balance', {account_id: ali.accountId});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});
    await ali.call(keypom, 'create_drop', {deposit_per_use: NEAR.parse('0.1').toString(), config}, {gas: LARGE_GAS});
    let simpleDropCreateCost = await keypom.view('get_user_balance', {account_id: ali.accountId});

    // Single key
    await ali.call(keypom, 'withdraw_from_balance', {account_id: ali.accountId});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});
    let aliBalInitial1: number  = await keypom.view('get_user_balance', {account_id: ali.accountId});

    let {keys: keys1, publicKeys: publicKeys1} = await generateKeyPairs(1);
    await ali.call(keypom, 'add_keys', {public_keys: publicKeys1, drop_id: "0"}, {gas: LARGE_GAS});
    await keypom.setKey(keys1[0]);
    const res1 = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
    displayFailureLog(res1);

    let numKeysRemaining = await keypom.view("get_key_supply_for_drop", {drop_id: "0"})
    t.is(numKeysRemaining == 0, true);
    let postSingleKey: number = await keypom.view('get_user_balance', {account_id: ali.accountId});
    let singleKeyCost = aliBalInitial1 - postSingleKey;
    
    // Ten keys
    await ali.call(keypom, 'withdraw_from_balance', {account_id: ali.accountId});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});
    let aliBalInitial10: number = await keypom.view('get_user_balance', {account_id: ali.accountId});

    let {keys: keys10, publicKeys: publicKeys10 } = await generateKeyPairs(10);
    await ali.call(keypom, 'add_keys', {public_keys: publicKeys10, drop_id: "0"}, {gas: LARGE_GAS});
    for(let i = 0; i < 10; i++){
        await keypom.setKey(keys10[i]);
        const res10 = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
        displayFailureLog(res10);
    }

    numKeysRemaining = await keypom.view("get_key_supply_for_drop", {drop_id: "0"})
    t.is(numKeysRemaining == 0, true);
    let postTenKeys: number = await keypom.view('get_user_balance', {account_id: ali.accountId});
    let tenKeyCost = aliBalInitial10 - postTenKeys;

    // Fifty keys
    await ali.call(keypom, 'withdraw_from_balance', {account_id: ali.accountId});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});
    let aliBalInitial50: number = await keypom.view('get_user_balance', {account_id: ali.accountId});

    let {keys: keys50, publicKeys: publicKeys50 } = await generateKeyPairs(50);
    await ali.call(keypom, 'add_keys', {public_keys: publicKeys50, drop_id: "0"}, {gas: LARGE_GAS});
    for(let i = 0; i < 50; i++){
        await keypom.setKey(keys50[i]);
        const res50 = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
        displayFailureLog(res50);
    }

    numKeysRemaining = await keypom.view("get_key_supply_for_drop", {drop_id: "0"})
    t.is(numKeysRemaining == 0, true);
    let postFiftyKeys: number = await keypom.view('get_user_balance', {account_id: ali.accountId});
    let fiftyKeyCost = aliBalInitial50 - postFiftyKeys;

    console.log(`SIMPLE DROP CREATION COSTS: ${simpleDropCreateCost}`)
    console.log(`1 KEY COSTS: ${singleKeyCost}`)
    console.log(`10 KEY COSTS: ${tenKeyCost}`)
    console.log(`50 KEY COSTS: ${fiftyKeyCost}`)
});

test('NFT Drop', async t => {
    //get Keypopm initial balance
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    //add 20 $NEAR to balance
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    //generate 1 key pair and 1 NFT
    let {keys: keys1, publicKeys: publicKeys1} = await generateKeyPairs(1);
    let nft: JsonNFTData = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }
    let config: DropConfig = {
        uses_per_key: 1,
    }
    await ali.call(keypom, 'create_drop', {
        public_keys: publicKeys1, 
        deposit_per_use: NEAR.parse("0.1").toString(),
        nft,
        config
    },{gas: LARGE_GAS});

    //minted and sent to minter
    await mintNFTs(minter, nftSeries, '0', 10);
    //transfer 10 NFTs from dropID 0 to Keypom from minter
    //nftSeries is the smart contract for said nft series, used to call mint on that contract
    //transfer from minter to keypom
    await sendNFTs(minter, ["1:1", "1:2", "1:3", "1:4", "1:5", "1:6", "1:7", "1:8", "1:9", "1:10"], keypom, nftSeries, "0");

    //registered uses here should be number of NFTs sent to Keypom
    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 10);

    //confirm Keypom now owns 10 NFTs. Note this is cumulative as it is using a view function on the NFT contract.
    //if keypom already owned NFTs on this contract and more were minted, it would be a sum of the old + new
    let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos: ', tokenInfos)
    t.is(tokenInfos.length, 10);

    //update the first 5 keys to have FAKs, claim 2 each
    for(let i = 0; i < 5; i++) {
        await keypom.setKey(keys1[i]);

        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : publicKeys1[5]}, {gas: WALLET_GAS});
        await keypom.call(keypom, 'claim', {account_id: `${i}.test.near`}, {gas: WALLET_GAS});

        //get token info and check that ${i}.test.near owns those NFTs that have been claimed
        let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: `${i}.test.near` });
        console.log(`account ID: ${i}.test.near`)
        console.log('tokenInfos: ', tokenInfos)
        t.is(tokenInfos[0].owner_id, `${i}.test.near`);
        t.is(tokenInfos[1].owner_id, `${i}.test.near`);
        t.is(tokenInfos.length, 2);
    }

    //Keypom should now own 0 NFTs from nftSeries as they have all been transfered
    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos at END for keypom: ', tokenInfos)
    t.is(tokenInfos.length, 0);

    //minter should also have 0 NFTs from nftSeries as it was initially transfered to Keypom.
    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: minter.accountId });
    console.log('tokenInfos at END for minter: ', tokenInfos)
    t.is(tokenInfos.length, 0);

    //registered uses and keys should now be both 0
    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 0);

    //withdraw remaining drop owner's Keypom balance
    await owner.call(keypom, 'delete_keys', {drop_id: "0"})
    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal !== "0");
    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal === "0");

    //make sure Keypom's $NEAR wallet balance did not decrease
    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
});

test('FT Drop', async t => {
    const { keypom, owner, ali, ftContract, minter } = t.context.accounts;
    //register keypom on ft contract
    await keypom.call(keypom, 'register_ft_contract', {account_id: ftContract.accountId}, {attachedDeposit: NEAR.parse("0.01")});
    //view keypom initial ft contract balance, should be null
    let storageBal = await ftContract.view('storage_balance_of', { account_id: keypom.accountId });
    console.log('storageBal: ', storageBal)
    t.not(storageBal, null);
    
    //store keypom's initial balance. keypomInitialBalance is defined as keypom.balance.available() as defined in beforeEach
    const keypomInitialBalance = t.context.keypomInitialBalance;

    //add 20NEAR to keypom's keypom wallet from its NEAR wallet
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    //generate 6 key pairs and create ft data containing contract ID, sender ID, balance per use
    let {keys, publicKeys} = await generateKeyPairs(6);
    //FTData struct defined in models, contains contract ID of ft, sender/minter ID, balance per use (here is 1NEAR) and ft storage
    let ft: FTData = {
        contract_id: ftContract.accountId,
        sender_id: minter.accountId,
        balance_per_use: oneGtNear.toString()
    }
    //2 uses per key
    let config: DropConfig = {
        uses_per_key: 2,
    }

    // Creating the FT drop with 5 keys, each with 2 uses per key
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys.slice(0, 5), 
        //deposit per use defines amount of near per registered use
        deposit_per_use: NEAR.parse("1").toString(),
        ft,
        config
    },{gas: LARGE_GAS});
    //creating this drop shuld cost owner 20 $NEAR as there is 1 $NEAR deposit per use and then 1 $NEAR per ft claimed

    // Get roughly the min for storing those token IDs
    //this should withdraw 10 $NEAR, owner's keypom balance should now be 0 $NEAR
    await owner.call(keypom, 'withdraw_from_balance', {});
    // Should kickback and refund minter's balance because this isn't enough NEAR for 1 registered use (defined above as 1 $NEAR)
    await sendFTs(minter, (oneGtNear/BigInt(2)).toString(), keypom, ftContract, "0");
    //minter here is the one calling ft_transfer_call and sending the ft to keypom (3rd parameter)

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    //since no FTs were loaded onto the contract, there should be 0 registered uses DESPITE the drop already being created
    t.is(viewFunctions.dropInformation?.registered_uses, 0);

    //minter balance should be 1000 $NEAR as no amount was transfered out yet
    let minterBal = await ftContract.view('ft_balance_of', { account_id: minter.accountId });
    console.log('minterBal: ', minterBal)
    t.is(minterBal, (oneGtNear * BigInt(1000)).toString());

    // Register all 10 uses --> 10 $NEAR / 1 $NEAR per registered use
    //number of uses is defined by automatically using [amount_sent/amount_per_use]
    await sendFTs(minter, (oneGtNear*BigInt(10)).toString(), keypom, ftContract, "0");

    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    //registered uses should now have increased to 10
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 10);

    //minter should now only have 1000-10 balance as they sent 10 $NEAR to keypom 
    minterBal = await ftContract.view('ft_balance_of', { account_id: minter.accountId });
    console.log('minterBal: ', minterBal)
    t.is(minterBal, (oneGtNear * BigInt(990)).toString());

    //keypom's ft contract balance of should now be 10 $NEAR after being sent it by minter
    let keypomBal = await ftContract.view('ft_balance_of', { account_id: keypom.accountId });
    console.log('keypomBal: ', keypomBal)
    t.is(keypomBal, (oneGtNear * BigInt(10)).toString());

    //create 5 users and claim a total of 2 $NEAR per user. 
    for(let i = 0; i < 5; i++) {
        await keypom.setKey(keys[i]);

        //claim 1 $NEAR each time
        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : publicKeys[5]}, {gas: WALLET_GAS});
        await keypom.call(keypom, 'claim', {account_id: `${i}.test.near`}, {gas: WALLET_GAS});

        //ensure the user balance is now 2 $NEAR 
        let newUserBal = await ftContract.view('ft_balance_of', { account_id: `${i}.test.near` });
        console.log(`account ID: ${i}.test.near`)
        console.log('newUserBal: ', newUserBal)
        t.is(newUserBal, (oneGtNear * BigInt(2)).toString());
    }

    //after keypom has distributed all the FTs, its balance should now be 0
    keypomBal = await ftContract.view('ft_balance_of', { account_id: keypom.accountId });
    console.log('keypomBal AFTER: ', keypomBal)
    t.is(keypomBal, "0");

    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    //a registered use was used each time claim or create_account_and_claim was called. This means with 5 users claiming twice, it should now be 0
    //in addition, the number of keys left in the drop should now be 0 as all 5 keys have used up their 2 uses and been automatically deleted
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 0);

    //CLARIFY THIS!!!!!
    //owner's balance should include some left over storage costs for the FTs?
    //could be pessimistic storage cost calcs leave some remaining balance on owner's account
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

// test('FC Drop', async t => {
//     const { keypom, nftContract, owner, ali, bob } = t.context.accounts;

//     const fcData: FCData = {
//         methods: [
//             [
//                 {
//                     receiver_id: nftContract.accountId,
//                     method_name: 'nft_mint',
//                     args: JSON.stringify({
//                         receiver_id: ali.accountId,
//                         token_id: '1',
//                         metadata: {}
//                     }),
//                     account_id_field: "account_id",
//                     key_id_field: "key_id",
//                     funder_id_field: "funder_id",
//                     drop_id_field: "drop_id",
//                     attached_deposit: NEAR.parse("1").toString(),
//                 }
//             ]
//         ]
//     }

//     const config: DropConfig = { 
//         uses_per_key: 1
//     }

//     let {keys, publicKeys} = await generateKeyPairs(1);
//     await ali.call(keypom, 'create_drop', {public_keys: publicKeys, deposit_per_use: NEAR.parse('1').toString(), fc: fcData, config}, {gas: LARGE_GAS, attachedDeposit: NEAR.parse('21').toString()});
//     await keypom.setKey(keys[0]);

//     // This should pass and none of the user provided args should be used.
//     const res = await keypom.callRaw(keypom, 'claim', {account_id: bob.accountId}, {gas: WALLET_GAS});
//     displayFailureLog(res);

//     let aliTokens: any = await nftContract.view('nft_tokens_for_owner', {account_id: ali.accountId});
//     console.log('aliTokens: ', aliTokens)
//     t.is(aliTokens.length, 1);
//     t.is(aliTokens[0].account_id, bob.accountId);
//     t.is(aliTokens[0].funder_id, ali.accountId);
//     t.is(aliTokens[0].key_id, "0");
//     t.is(aliTokens[0].drop_id, "0");
// });