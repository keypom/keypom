import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, functionCall, generateKeyPairs } from "../utils/general";
import { keypom_args, nftMetadata, nftSeriesMetadata } from "./utils/nft-utils";
import { generatePasswordsForKey } from "./utils/pwUtils";

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
    const funder = await root.createSubAccount('funder');
    const minter = await root.createSubAccount('minter');

    // Add 10k $NEAR to funder's account
    await funder.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: "0"});

    // Mint the NFT
    await nftSeries.call(nftSeries, 'create_series', { mint_id: 0, metadata: nftMetadata }, { attachedDeposit: NEAR.parse("1").toString() });
    await nftSeries.call(nftSeries, 'nft_mint', { mint_id: '0', receiver_id: minter, keypom_args }, { attachedDeposit: NEAR.parse("1").toString() });

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, funder, ali, minter, nftSeries };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Null Claim', async t => {
    //get Keypopm initial balance
    const { keypom, funder, ali, nftSeries, minter } = t.context.accounts;

    //add 20 $NEAR to balance
    console.log("adding to balance");
    await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

    const dropId = "drop-id";

    await functionCall({
        signer: funder,
        receiver: keypom,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data_for_all_uses: {
                assets: [null],
                num_uses: 1
            },
            public_keys: []
        },
        attachedDeposit: NEAR.parse("20").toString()
    })

    let numKeys = 1;
    let {keys, publicKeys} = await generateKeyPairs(numKeys);

    let basePassword = 'mypassword1';
    //generates an array of hash(hash(basePassword + publicKeys[i])) --> all PWs for all key use
    let password_by_use = generatePasswordsForKey(publicKeys[0], [1], basePassword);
    
    // Create an array of size numKeys that's entirely empty
    let data_for_keys = new Array(numKeys).fill(null);
    data_for_keys[0] = {
        password_by_use
    }
    await functionCall({
        signer: funder,
        receiver: keypom,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            data_for_keys,
            public_keys: publicKeys
        },
        attachedDeposit: NEAR.parse("20").toString()
    })
});