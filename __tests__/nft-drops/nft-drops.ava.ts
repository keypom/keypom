import anyTest, { TestFn } from "ava";
import { Account, NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, JsonKeyInfo, JsonNFTData, JsonToken } from "../utils/types";
import { keypom_args, mintNFTs, nftMetadata, nftSeriesMetadata, sendNFTs } from "./utils/nft-utils";

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

    // Deploy all 3 contracts to 2 dev accounts; the NFT series and keypom
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    const nftSeries = await root.devDeploy(`./__tests__/ext-wasm/nft-series.wasm`);

    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    await nftSeries.call(nftSeries, 'new', { owner_id: nftSeries, metadata: nftSeriesMetadata });

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const minter = await root.createSubAccount('minter');

    // Add 10k $NEAR to owner's account
    await owner.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: "0"});

    // Mint the NFT
    await nftSeries.call(nftSeries, 'create_series', { mint_id: 0, metadata: nftMetadata }, { attachedDeposit: NEAR.parse("1").toString() });
    await nftSeries.call(nftSeries, 'nft_mint', { mint_id: '0', receiver_id: minter, keypom_args }, { attachedDeposit: NEAR.parse("1").toString() });
    
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: owner.accountId });
    await keypom.call(keypom, 'add_to_refund_allowlist', { account_id: ali.accountId });
    
    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    
    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, owner, ali, minter, nftSeries };
    t.context.keypomInitialBalance = keypomBalance.available;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Claim Multi NFT Drop And Ensure Keypom Balance Increases', async t => {
    //get Keypopm initial balance
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    //add 20 $NEAR to balance
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    //generate 6 key pairs and JsonNFTData
    let {keys, publicKeys} = await generateKeyPairs(6);
    let nft: JsonNFTData = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }
    //2 uses per key
    let config: DropConfig = {
        uses_per_key: 2,
    }

    // Creating the NFT drop with 5 keys, 10 uses
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys.slice(0, 5), 
        deposit_per_use: NEAR.parse("1").toString(),
        nft,
        config
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs (0.009 $NEAR, 0.001 $NEAR per NFT tokenID) and add to owner's Keypom Balance
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("0.009").toString()});

    // Mint another 9 NFTs (there is now 10 in total, 1 was minted in the beforeEach setup)
    //minted and sent to minter
    await mintNFTs(minter, nftSeries, '0', 9);
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
        await keypom.setKey(keys[i]);

        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : publicKeys[5]}, {gas: WALLET_GAS});
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

test('OverRegister NFTs and add multi use key later', async t => {
    //get Keypom initial balance
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    //add 10 $NEAR to owner's balance
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 2 keypairs, make JsonNFTData with genuine minter and nftSeries
    let {keys, publicKeys} = await generateKeyPairs(2);
    let nft: JsonNFTData = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }

    //10 uses per key
    let config: DropConfig = {
        uses_per_key: 10,
    }

    // Creating the NFT drop with NO keys
    await owner.call(keypom, 'create_drop', {
        deposit_per_use: NEAR.parse("1").toString(),
        nft,
        config
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("0.009").toString()});

    // Mint another 9 NFTs (there is now 10 in total)
    await mintNFTs(minter, nftSeries, '0', 9);
    //send 10 the 10 NFTs to Keypom from minter
    await sendNFTs(minter, ["1:1", "1:2", "1:3", "1:4", "1:5", "1:6", "1:7", "1:8", "1:9", "1:10"], keypom, nftSeries, "0");

    //NO keys, 10 registered uses as there are 10 NFTs associated with DropID 0
    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.keysForDrop?.length, 0);
    t.is(viewFunctions.dropInformation?.registered_uses, 10);

    //make sure Keypom owns 10 NFTs
    let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos: ', tokenInfos)
    t.is(tokenInfos.length, 10);

    //add 20 $NEAR to owner's balance, then add a key to the drop
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});
    await owner.call(keypom, 'add_keys', {drop_id: '0', public_keys: [publicKeys[0]]}, {gas: LARGE_GAS});
    await keypom.setKey(keys[0]);
    for(let i = 0; i < 5; i++) {
        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : publicKeys[1]}, {gas: WALLET_GAS});
        await keypom.call(keypom, 'claim', {account_id: `${i}.test.near`}, {gas: WALLET_GAS});

        //make sre each ${i}.test.near owns 2 NFTs
        let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: `${i}.test.near` });
        console.log(`account ID: ${i}.test.near`)
        console.log('tokenInfos: ', tokenInfos)
        t.is(tokenInfos[0].owner_id, `${i}.test.near`);
        t.is(tokenInfos[1].owner_id, `${i}.test.near`);
        t.is(tokenInfos.length, 2);
    }
    //all 10 uses for publicKeys[0] should now be used. The key should be auto deleted from the drop

    //Keypom should no longer have any NFTs as they were all transfered
    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos at END for keypom: ', tokenInfos)
    t.is(tokenInfos.length, 0);

    //minter sent all of its NFTs to Keypom and hence should have 0
    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: minter.accountId });
    console.log('tokenInfos at END for minter: ', tokenInfos)
    t.is(tokenInfos.length, 0);

    //no more registered uses, the only key that was added to the drop has also used all 10 of its uses. 
    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 0);

    //delete keys and clear balance
    await owner.call(keypom, 'delete_keys', {drop_id: "0"})
    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal !== "0");
    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal === "0");

    //make sure Keypom has not lost $NEAR through this process.
    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
});

test('Not enough funder balance stage 2', async t => {
    //stage 2 refers to adding NFTs to the drop. Here, we will withdraw the funder's balance before sending NFTs
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(2);
    let nft: JsonNFTData = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }

    // Creating the drop that should be deleted
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        nft,
    },{gas: LARGE_GAS});

    //drop owner needs to pay for storage of the NFTs that are sent to their drop.
    //Thus if we withdraw their balance before sending them NFTs, it should fail
    await owner.call(keypom, 'withdraw_from_balance', {});
    await sendNFTs(minter, ["1:1"], keypom, nftSeries, "0");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation B4: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop B4: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);

    //since sending the NFT failed, Keypom is NOT the owner; instead it rolled back and restored minter as the owner
    let tokenInfo: JsonToken = await nftSeries.view('nft_token', { token_id: "1:1" });
    console.log('tokenInfo: ', tokenInfo)
    t.is(tokenInfo.owner_id, minter.accountId);

    //adding just enough balance to owner's accont so that they can pay for storage of the NFT they are being sent.
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("0.001").toString()});
    await sendNFTs(minter, ["1:1"], keypom, nftSeries, "0");

    //now owner should own the NFT and there should be 1 registered use for the drop
    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 1);

    tokenInfo = await nftSeries.view('nft_token', { token_id: "1:1" });
    console.log('tokenInfo: ', tokenInfo)
    t.is(tokenInfo.owner_id, keypom.accountId);
});

test('Deleting Keys and Drop', async t => {
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    //generate 6 key pairs, create normal JsonNFTData
    let {keys, publicKeys} = await generateKeyPairs(6);
    let nft: JsonNFTData = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }

    //adding 100 $NEAR to owner's balance, then create drop with the above NFTData
    console.log("adding to balance");
    // How much does it cost to create a drop?
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    await owner.call(keypom, 'create_drop', {
        deposit_per_use: NEAR.parse("1").toString(),
        nft,
    },{gas: LARGE_GAS});
    //get balance after creating drop
    let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating key: ', ownerBal)
    let netCostCreatingDrop = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostCreatingDrop: ', netCostCreatingDrop.toString())

    // Measure how much $NEAR it costs to add a single key, should be any storage + deposit_per_use
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
    //delete_on_empty is true by default, this makes this line delete the drop
    await owner.call(keypom, 'delete_keys', {drop_id: "0"});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('new ownerBal after del drop: ', ownerBal)
    t.is(NEAR.from(ownerBal).toString(), netCostCreatingDrop.toString());

    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal === "0");

    //keypom's NEAR wallet balances
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
        nft,
    },{gas: LARGE_GAS});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating key: ', ownerBal)
    let netCostCreatingDropWithOneKey = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostCreatingDropWithOneKey: ', netCostCreatingDropWithOneKey.toString())
    t.is(netCostCreatingDropWithOneKey.toString(), netCostAddingOneKey.add(netCostCreatingDrop).toString());
});

test('Refunding Assets and Deleting Multi Use Keys and Drops', async t => {
    //get Keypopm's initial $NEAR wallet balance for reference
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    //generate 2 keypairs, make normal NFT config
    let {keys, publicKeys} = await generateKeyPairs(2);
    let nft: JsonNFTData = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }

    //10 uses per key
    let config: DropConfig = {
        uses_per_key: 10,
    }

    //adding 100 $NEAR to owner's Keypom wallet and then creating drop using 1 key.
    //Use this process to figure out cost of creating a drop with 1 key
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("100").toString()});
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        nft,
        config
    },{gas: LARGE_GAS});
    let ownerBal: string = await keypom.view('get_user_balance', {account_id: owner});
    console.log('ownerBal after creating drop with key: ', ownerBal)
    let netCost = NEAR.parse("100").sub(NEAR.from(ownerBal));
    console.log('netCostCreatingDrop: ', netCost.toString())

    // Send 9 NFT registers to the contract
    await mintNFTs(minter, nftSeries, '0', 9);
    await sendNFTs(minter, ["1:1", "1:2", "1:3", "1:4", "1:5", "1:6", "1:7", "1:8", "1:9", "1:10"], keypom, nftSeries, "0");

    await keypom.setKey(keys[0]);
    
    // Use the key 5 out of 10 times
    for(let i = 0; i < 5; i++) {
        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : publicKeys[1]}, {gas: WALLET_GAS});

        let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: `${i}.test.near` });
        console.log(`account ID: ${i}.test.near`)
        console.log('tokenInfos: ', tokenInfos)
        t.is(tokenInfos[0].owner_id, `${i}.test.near`);
        t.is(tokenInfos.length, 1);
    }

    //Keypom should now only have 5 of the original 10 NFTs under its ownership
    let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    t.is(tokenInfos.length, 5);

    //key is not fully used, is not deleted
    //5 NFTs left, 5 registered uses left
    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 5);
    t.is(viewFunctions.keysForDrop?.length, 1);

    //refund_assets sends the assets from drop X back to minter/sender
    //no $NEAR is refunded back to the owner for this
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'refund_assets', {drop_id: "0"}, {gas: LARGE_GAS});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.is(ownerBal, "0");

    //after refunding assets, there are no more registered uses.
    //However, keys are still 1 as it was never deleted nor claimed
    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: FINAL ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: FINAL ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);
    t.is(viewFunctions.keysForDrop?.length, 1);

    //keypom surrendered its last 5 NFTs
    tokenInfos= await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    t.is(tokenInfos.length, 0);

    //minter should have been refunding the remaining 5 assets when refund_assets was called
    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: minter.accountId });
    t.is(tokenInfos.length, 5);

    //nmaking sure Keypom does not lose value during this process
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'delete_keys', {drop_id: "0"});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal > "0");
    await owner.call(keypom, 'withdraw_from_balance', {});

    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
});

test('Anyone can send NFTs since sender is optional', async t => {
    //get Keypom initial balance
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    //add 10 $NEAR to owner's balance
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    //generate 2 keypairs, make JsonNFTData with genuine minter and nftSeries
    let {keys, publicKeys} = await generateKeyPairs(2);
    let nft: JsonNFTData = {
        contract_id: nftSeries.accountId
    }

    //10 uses per key
    let config: DropConfig = {
        uses_per_key: 10,
    }

    // Creating the NFT drop with NO keys
    await owner.call(keypom, 'create_drop', {
        deposit_per_use: NEAR.parse("1").toString(),
        nft,
        config
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs
    await owner.call(keypom, 'withdraw_from_balance', {});

    // Mint an NFT
    await mintNFTs(minter, nftSeries, '0', 9);
    //send the NFT to the contract (this should throw since the owner doesn't have enough balance)
    await sendNFTs(minter, ["1:1"], keypom, nftSeries, "0");

    //NO keys, 10 registered uses as there are 10 NFTs associated with DropID 0
    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.keysForDrop?.length, 0);
    t.is(viewFunctions.dropInformation?.registered_uses, 0);

    //make sure Keypom owns no NFTs and it successfully threw an error
    let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos: ', tokenInfos)
    t.is(tokenInfos.length, 0);

    //add 20 $NEAR to owner's balance, then add a key to the drop
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    //send the NFT to the contract (this should not throw anymore since $NEAR was added)
    await sendNFTs(minter, ["1:1"], keypom, nftSeries, "0");

    //NO keys, 10 registered uses as there are 10 NFTs associated with DropID 0
    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: "0"
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.keysForDrop?.length, 0);
    t.is(viewFunctions.dropInformation?.registered_uses, 1);

    //make sure Keypom owns 1 NFT
    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos: ', tokenInfos)
    t.is(tokenInfos.length, 1);
    
    //delete keys and clear balance
    await owner.call(keypom, 'refund_assets', {drop_id: "0"}, {gas: LARGE_GAS});
    await owner.call(keypom, 'delete_keys', {drop_id: "0"})
    let ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal !== "0");
    await owner.call(keypom, 'withdraw_from_balance', {});
    ownerBal = await keypom.view('get_user_balance', {account_id: owner});
    t.assert(ownerBal === "0");

    //make sure Keypom has not lost $NEAR through this process.
    let keypomBalance = await keypom.balance();
    console.log('keypom available FINAL: ', keypomBalance.available.toString())
    t.assert(keypomBalance.available > keypomInitialBalance);
});