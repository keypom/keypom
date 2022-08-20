import { Worker, NearAccount, NEAR, KeyPairEd25519, tGas, KeyPair } from "near-workspaces";
import anyTest, { TestFn } from "ava";
import { assertBalanceChange, createSeries, defaultCallOptions, DEFAULT_DEPOSIT, DEFAULT_GAS, generateKeyPairs, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/utils";
import { JsonDrop, JsonKeyInfo, TokenMetadata } from "../utils/types";

const METADATA = {
    "title": "Linkdropped Go Team NFT",
    "description": "Testing Linkdrop NFT Go Team Token",
    "media": "https://bafybeiftczwrtyr3k7a2k4vutd3amkwsmaqyhrdzlhvpt33dyjivufqusq.ipfs.dweb.link/goteam-gif.gif",
    "media_hash": null,
    "copies": 100,
    "issued_at": null,
    "expires_at": null,
    "starts_at": null,
    "updated_at": null,
    "extra": null,
    "reference": null,
    "reference_hash": null
};

let config = {
    uses_per_key: 3,
    on_claim_refund_deposit: true
}

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
}>;

test.beforeEach(async (t) => {
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy the keypom contract.
    const keypom = await root.devDeploy(`./out/main.wasm`);
    const nftSeries = await root.devDeploy(`./__tests__/ext-wasm/nft-series.wasm`);

    // Init the contract
    await keypom.call(keypom, 'new', { root_account: 'testnet', owner_id: keypom });
    await nftSeries.call(nftSeries, 'new_default_meta', { owner_id: nftSeries });

    // Test users
    const ali = await root.createSubAccount('ali');
    const owner = await root.createSubAccount('owner');
    const bob = await root.createSubAccount('bob');

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, nftSeries, owner, ali, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Create All Drops', async t => {
    const { keypom, nftSeries, owner, ali, bob} = t.context.accounts;

    let balance = await keypom.balance();
    console.log('available: ', balance.available.toString())
    console.log('staked: ', balance.staked.toString())
    console.log('stateStaked: ', balance.stateStaked.toString())
    console.log('total: ', balance.total.toString())
    // await keypom.updateAccount({
    //     amount: balance.staked.toString()
    // })

    let fc_data = {
        methods: [
            null,
            null,
            [{
                receiver_id: nftSeries,
                method_name: "nft_mint",
                args: "",
                attached_deposit: NEAR.parse("0.01").toString(),
                account_id_field: "receiver_id",
                drop_id_field: "id"
            }]
        ]
    }

    await owner.updateAccount({
        amount: NEAR.parse('1000 N').toString()
    })
    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("79").toString()});

    // Loop through 5 times and create a drop with 100 keys each time
    for (let i = 0; i < 5; i++) {
        console.log("Creating 100 keys to create drop")
        let {publicKeys} = await generateKeyPairs(100);

        await owner.call(keypom, 'create_drop', {
            public_keys: publicKeys, 
            deposit_per_use: NEAR.parse('5 mN').toString(),
            fc_data,
            config,
        },{gas: LARGE_GAS});

        console.log("Creating 100 keys to add to drop")
        publicKeys = await (await generateKeyPairs(100)).publicKeys;
        
        await owner.call(keypom, 'add_keys', {
            public_keys: publicKeys, 
            drop_id: i
        },{gas: LARGE_GAS});
    }

    let result = await queryAllViewFunctions({
        contract: keypom, 
        drop_id: 0, 
        account_id: owner.accountId
    });
    console.log('result: ', result)

    balance = await keypom.balance();
    console.log('available: ', balance.available.toString())
    console.log('staked: ', balance.staked.toString())
    console.log('stateStaked: ', balance.stateStaked.toString())
    console.log('total: ', balance.total.toString())
});

// test('Fully Claim 1 key', async t => {
//     const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
//     let { keys, publicKeys } = await generateKeyPairs(1);

//     console.log("Creating series");
//     await createSeries({
//         account: nftSeries,
//         nftContract: nftSeries,
//         metadatas: [METADATA],
//         ids: [0]
//     })

//     console.log("adding to balance");
//     await owner.call(keypom, 'add_to_balance', {}, { attachedDeposit: NEAR.parse("8").toString() });

//     let fc_data = {
//         methods: [
//             null,
//             null,
//             [{
//                 receiver_id: nftSeries,
//                 method_name: "nft_mint",
//                 args: "",
//                 attached_deposit: NEAR.parse("0.01").toString(),
//                 account_id_field: "receiver_id",
//                 drop_id_field: "id"
//             }]
//         ]
//     }

//     console.log("creating drop");
//     await owner.call(keypom, 'create_drop', {
//         public_keys: [publicKeys[0]],
//         deposit_per_use: NEAR.parse('5 mN').toString(),
//         fc_data,
//         config,
//     }, { gas: LARGE_GAS });

//     bob.updateAccount({
//         amount: "0"
//     });

//     await keypom.setKey(keys[0]);

//     // await keypom.updateAccessKey(
//     //     keys[0],  // public key
//     //     {
//     //         nonce: 0,
//     //         permission: {
//     //             FunctionCall: {
//     //                 allowance: "18762630063718400000000",
//     //                 receiver_id: keypom.accountId,
//     //                 method_names: ["claim", "create_account_and_claim"],
//     //             }
//     //         }
//     //     }
//     // )

//     await keypom.call(keypom, 'claim', { account_id: bob }, { gas: WALLET_GAS });
//     await keypom.call(keypom, 'claim', { account_id: bob }, { gas: WALLET_GAS });
//     let { keys: keys2, publicKeys: pks2 } = await generateKeyPairs(1);
//     //await keypom.call(keypom, 'create_account_and_claim', {new_account_id: 'benji.test.near', new_public_key : pks2[0]}, {gas: WALLET_GAS});
//     await keypom.call(keypom, 'claim', { account_id: bob }, { gas: WALLET_GAS });
//     let bobInfo = await bob.balance();
//     console.log('bobInfo: ', bobInfo)
//     let res = await nftSeries.view('nft_tokens', {});
//     console.log('res: ', res)
// });

// test('Ticketing Injected Fields', async t => {
// });