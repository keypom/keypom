import anyTest, { TestFn } from "ava";
import { Account, NEAR, NearAccount, Worker } from "near-workspaces";
import { generateKeyPairs, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { JsonKeyInfo, JsonToken } from "../utils/types";
import { injected_fields, mintNFTs, nftMetadata, nftSeriesMetadata, sendNFTs } from "./utils/nft-utils";

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
    const nftSeries = await root.devDeploy(`./__tests__/ext-wasm/nft-series.wasm`);

    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom });
    await nftSeries.call(nftSeries, 'new', { owner_id: nftSeries, metadata: nftSeriesMetadata });
    
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
    await nftSeries.call(nftSeries, 'create_series', { mint_id: 0, metadata: nftMetadata }, { attachedDeposit: NEAR.parse("1").toString() });
    await nftSeries.call(nftSeries, 'nft_mint', { mint_id: '0', receiver_id: minter, injected_fields }, { attachedDeposit: NEAR.parse("1").toString() });

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
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(6);
    let nft_data = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }

    // Creating the NFT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: publicKeys.slice(0, 5), 
        deposit_per_use: NEAR.parse("1").toString(),
        nft_data,
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("0.005").toString()});

    // Mint another 4 NFTs (there is now 5 in total)
    await mintNFTs(minter, nftSeries, '0', 4);
    await sendNFTs(minter, ["1:1", "1:2", "1:3", "1:4", "1:5"], keypom, nftSeries, "0");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: 0
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 5);

    let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos: ', tokenInfos)
    t.is(tokenInfos.length, 5);

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

        let tokenInfo: JsonToken = await nftSeries.view('nft_token', { token_id: `1:${5-i}` });
        console.log(`account ID: ${i}.test.near`)
        console.log('tokenInfo: ', tokenInfo)
        t.is(tokenInfo.owner_id, `${i}.test.near`);
    }

    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos at END for keypom: ', tokenInfos)
    t.is(tokenInfos.length, 0);

    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: minter.accountId });
    console.log('tokenInfos at END for minter: ', tokenInfos)
    t.is(tokenInfos.length, 0);

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

test('OverRegister NFTs and add multi use key later', async t => {
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    const keypomInitialBalance = t.context.keypomInitialBalance;

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(2);
    let nft_data = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }

    let config = {
        uses_per_key: 5,
    }

    // Creating the NFT drop with 5 keys
    await owner.call(keypom, 'create_drop', {
        public_keys: [], 
        deposit_per_use: NEAR.parse("1").toString(),
        nft_data,
        config
    },{gas: LARGE_GAS});

    // Get roughly the min for storing those token IDs
    await owner.call(keypom, 'withdraw_from_balance', {});
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("0.005").toString()});

    // Mint another 4 NFTs (there is now 5 in total)
    await mintNFTs(minter, nftSeries, '0', 4);
    await sendNFTs(minter, ["1:1", "1:2", "1:3", "1:4", "1:5"], keypom, nftSeries, "0");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: 0
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.keysForDrop?.length, 0);
    t.is(viewFunctions.dropInformation?.registered_uses, 5);

    let tokenInfos: JsonToken[] = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos: ', tokenInfos)
    t.is(tokenInfos.length, 5);

    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});
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

        let tokenInfo: JsonToken = await nftSeries.view('nft_token', { token_id: `1:${5-i}` });
        console.log(`account ID: ${i}.test.near`)
        console.log('tokenInfo: ', tokenInfo)
        t.is(tokenInfo.owner_id, `${i}.test.near`);
    }

    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: keypom.accountId });
    console.log('tokenInfos at END for keypom: ', tokenInfos)
    t.is(tokenInfos.length, 0);

    tokenInfos = await nftSeries.view('nft_tokens_for_owner', { account_id: minter.accountId });
    console.log('tokenInfos at END for minter: ', tokenInfos)
    t.is(tokenInfos.length, 0);

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

test('Not enough funder balance stage 2', async t => {
    const { keypom, owner, ali, nftSeries, minter } = t.context.accounts;
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("10").toString()});

    let {keys, publicKeys} = await generateKeyPairs(2);
    let nft_data = {
        sender_id: minter.accountId,
        contract_id: nftSeries.accountId
    }

    // Creating the drop that should be deleted
    await owner.call(keypom, 'create_drop', {
        public_keys: [publicKeys[0]], 
        deposit_per_use: NEAR.parse("1").toString(),
        nft_data,
    },{gas: LARGE_GAS});

    await owner.call(keypom, 'withdraw_from_balance', {});
    await sendNFTs(minter, ["1:1"], keypom, nftSeries, "0");

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: 0
    });
    console.log('viewFunctions.dropInformation B4: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop B4: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 0);

    let tokenInfo: JsonToken = await nftSeries.view('nft_token', { token_id: "1:1" });
    console.log('tokenInfo: ', tokenInfo)
    t.is(tokenInfo.owner_id, minter.accountId);

    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("0.001").toString()});
    await sendNFTs(minter, ["1:1"], keypom, nftSeries, "0");

    viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
        drop_id: 0
    });
    console.log('viewFunctions.dropInformation: ', viewFunctions.dropInformation)
    console.log('viewFunctions.keysForDrop: ', viewFunctions.keysForDrop)
    t.is(viewFunctions.dropInformation?.registered_uses, 1);

    tokenInfo = await nftSeries.view('nft_token', { token_id: "1:1" });
    console.log('tokenInfo: ', tokenInfo)
    t.is(tokenInfo.owner_id, keypom.accountId);
});