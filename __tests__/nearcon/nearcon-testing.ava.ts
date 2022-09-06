import anyTest, { TestFn } from "ava";
import { KeyPair, NEAR, NearAccount, Worker } from "near-workspaces";
import { generateKeyPairs, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { ticketDistro1, ticketDistro2 } from "./distro";
import { createDistro, dropConfig, getNEARConFCData, nftMetadata, nftSeriesMetadata } from "./nearconUtils";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
}>;

test.beforeEach(async (t) => {
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy all 3 contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    const nftSeries = await root.devDeploy(`./__tests__/ext-wasm/nft-series.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    
    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom });
    await nftSeries.call(nftSeries, 'new', { owner_id: nftSeries, metadata: nftSeriesMetadata });
    
    // Add Keypom as an approved minter
    await nftSeries.call(nftSeries, 'add_approved_minter', { account_id: keypom });

    let keypomBalance = await keypom.balance();
    console.log('keypom available INITIAL: ', keypomBalance.available.toString())
    console.log('keypom staked INITIAL: ', keypomBalance.staked.toString())
    console.log('keypom stateStaked INITIAL: ', keypomBalance.stateStaked.toString())
    console.log('keypom total INITIAL: ', keypomBalance.total.toString())

    let nftBalance = await nftSeries.balance();
    console.log('nftSeries available INITIAL: ', nftBalance.available.toString())
    console.log('nftSeries staked INITIAL: ', nftBalance.staked.toString())
    console.log('nftSeries stateStaked INITIAL: ', nftBalance.stateStaked.toString())
    console.log('nftSeries total INITIAL: ', nftBalance.total.toString())

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

// test('Test Stage 1 Went Well', async t => {
//     const { keypom, nftSeries, owner, ali, bob} = t.context.accounts;

//     // Check how many keys are in the ticketDistro object
//     let numSponsors = Object.keys(ticketDistro).length;
    
//     // Get how many keys were made in total
//     let numTotalKeys = 0;
//     for (let [sponsor, tickets] of Object.entries(ticketDistro)) {
//         let totalNumTixForSponsor = tickets.reduce((partialSum, a) => partialSum + a, 0);
//         numTotalKeys += totalNumTixForSponsor;
//     }

//     let mint_id = 0;
//     for (let [sponsor, tickets] of Object.entries(ticketDistro)) {
//         let totalNumTix = tickets.reduce((partialSum, a) => partialSum + a, 0);

//         let result = await queryAllViewFunctions({
//             contract: keypom, 
//             drop_id: mint_id, 
//             account_id: owner.accountId
//         });

//         let jsonDrop: JsonDrop = result.dropInformation!;
//         console.log('jsonDrop: ', jsonDrop)
//         t.is(jsonDrop.drop_id, mint_id);
//         t.is(jsonDrop.owner_id, owner.accountId);
//         t.is(jsonDrop.deposit_per_use, NEAR.parse('20 mN').toString());
//         t.is(jsonDrop.registered_uses, totalNumTix * 3);
//         t.is(jsonDrop.required_gas, tGas(100));
//         t.is(jsonDrop.next_key_id, totalNumTix);

//         t.is(result.keySupplyForDrop, totalNumTix);

//         t.is(result.keyTotalSupply, numTotalKeys.toString());

//         t.deepEqual(result.dropSupplyForOwner, numSponsors);

//         t.deepEqual(jsonDrop.config, {
//             uses_per_key: 3,
//             start_timestamp: null,
//             throttle_timestamp: null,
//             on_claim_refund_deposit: true,
//             claim_permission: null,
//             drop_root: null,
//         });

//         let seriesInfo = await nftSeries.view('get_series_info', {mint_id});
//         console.log('seriesInfo: ', seriesInfo)
//         //@ts-ignore
//         t.is(seriesInfo.mint_id, mint_id);
//         //@ts-ignore
//         t.is(seriesInfo.series_id, mint_id+1);

//         mint_id += 1;
//     }

//     let allSeries = await nftSeries.view('get_series');
    
//     // @ts-ignore
//     t.is(allSeries.length, numSponsors);
//     console.log('allSeries: ', allSeries)

//     let nftBalance = await nftSeries.balance();
//     console.log('available: ', nftBalance.available.toString())
//     console.log('staked: ', nftBalance.staked.toString())
//     console.log('stateStaked: ', nftBalance.stateStaked.toString())
//     console.log('total: ', nftBalance.total.toString())
// });

// test('Fully Claim all keys', async t => {
//     const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
//     let keyPairsForSponsors = await createDistro(ticketDistro2, owner, keypom, nftSeries, "1");
//     console.log('keyPairsForSponsors: ', keyPairsForSponsors)
//     let { keys: keys2, publicKeys: pks2 } = await generateKeyPairs(1);

//     let curBal = await keypom.view('get_user_balance', {account_id: owner});
//     console.log('curBal before: ', curBal)

//     let nonce = 0;
//     // Iterate through each sponsor in the keyPairsForSponsors object
//     for (let [sponsor, keys] of Object.entries(keyPairsForSponsors)) {
//         console.log(`Claiming ${keys.length} keys for ${sponsor}`);

//         for(let i = 0; i < keys.length; i++) {
//             await keypom.setKey(keys[i]);
//             await keypom.updateAccessKey(
//                 keys[i],  // public key
//                 {
//                     nonce: 0,
//                     permission: 'FullAccess'
//                 }
//             )

//             await keypom.call(keypom, 'claim', { account_id: bob }, { gas: WALLET_GAS });
//             await keypom.call(keypom, 'claim', { account_id: bob }, { gas: WALLET_GAS });
//             await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${nonce}-${i}.test.near`, new_public_key : pks2[0]}, {gas: WALLET_GAS});
//         }
//         nonce += 1;
//     }
    
//     curBal = await keypom.view('get_user_balance', {account_id: owner});
//     console.log('curBal after: ', curBal)

//     let keypomInfo = await keypom.balance();
//     console.log('keypom available: ', keypomInfo.available.toString())
//     console.log('keypom staked: ', keypomInfo.staked.toString())
//     console.log('keypom stateStaked: ', keypomInfo.stateStaked.toString())
//     console.log('keypom total: ', keypomInfo.total.toString())

//     let nftSeriesInfo = await nftSeries.balance();
//     console.log('nft Series available: ', nftSeriesInfo.available.toString())
//     console.log('nft Series staked: ', nftSeriesInfo.staked.toString())
//     console.log('nft Series stateStaked: ', nftSeriesInfo.stateStaked.toString())
//     console.log('nft Series total: ', nftSeriesInfo.total.toString())

//     let getSeries = await nftSeries.view('get_series', {});
//     console.log('getSeries: ', getSeries)
    
//     let supplySeries = await nftSeries.view('get_supply_series', {});
//     console.log('supplySeries: ', supplySeries)

//     let nftTotalSupply = await nftSeries.view('nft_total_supply', {});
//     console.log('nftTotalSupply: ', nftTotalSupply)

//     let viewFunctions = await queryAllViewFunctions({
//         contract: keypom, 
//         account_id: owner.accountId,
//     });
//     console.log('viewFunctions: ', viewFunctions)

//     nonce = 0;
//     for (let [sponsor, keys] of Object.entries(keyPairsForSponsors)) {
//         let nftTotalSupply = await nftSeries.view('nft_supply_for_series', {mint_id: nonce});
//         console.log(`nftTotalSupply for ${sponsor}: ${nftTotalSupply}`);
//         nonce += 1;
//     }
// });

// test('Claim 1 with invalid expected uses', async t => {
//     const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
//     let keyPairsForSponsors = await createDistro(ticketDistro1, owner, keypom, nftSeries, "1");
//     console.log('keyPairsForSponsors: ', keyPairsForSponsors)
//     let { keys: keys2, publicKeys: pks2 } = await generateKeyPairs(1);

//     let curBal = await keypom.view('get_user_balance', {account_id: owner});
//     console.log('curBal before: ', curBal)

//     let nonce = 0;
//     // Iterate through each sponsor in the keyPairsForSponsors object
//     for (let [sponsor, keys] of Object.entries(keyPairsForSponsors)) {
//         console.log(`Claiming ${keys.length} keys for ${sponsor}`);

//         for(let i = 0; i < keys.length; i++) {
//             await keypom.setKey(keys[i]);
//             await keypom.updateAccessKey(
//                 keys[i],  // public key
//                 {
//                     nonce: 0,
//                     permission: 'FullAccess'
//                 }
//             )

//             await keypom.call(keypom, 'claim', { account_id: bob, expected_uses: 3 }, { gas: WALLET_GAS });
//             await keypom.call(keypom, 'claim', { account_id: bob, expected_uses: 3 }, { gas: WALLET_GAS });
//         }
//         nonce += 1;
//     }
    
//     curBal = await keypom.view('get_user_balance', {account_id: owner});
//     console.log('curBal after: ', curBal)

//     let keypomInfo = await keypom.balance();
//     console.log('keypom available: ', keypomInfo.available.toString())
//     console.log('keypom staked: ', keypomInfo.staked.toString())
//     console.log('keypom stateStaked: ', keypomInfo.stateStaked.toString())
//     console.log('keypom total: ', keypomInfo.total.toString())

//     let nftSeriesInfo = await nftSeries.balance();
//     console.log('nft Series available: ', nftSeriesInfo.available.toString())
//     console.log('nft Series staked: ', nftSeriesInfo.staked.toString())
//     console.log('nft Series stateStaked: ', nftSeriesInfo.stateStaked.toString())
//     console.log('nft Series total: ', nftSeriesInfo.total.toString())

//     let getSeries = await nftSeries.view('get_series', {});
//     console.log('getSeries: ', getSeries)
    
//     let supplySeries = await nftSeries.view('get_supply_series', {});
//     console.log('supplySeries: ', supplySeries)

//     let nftTotalSupply = await nftSeries.view('nft_total_supply', {});
//     console.log('nftTotalSupply: ', nftTotalSupply)

//     let viewFunctions = await queryAllViewFunctions({
//         contract: keypom, 
//         account_id: owner.accountId,
//     });
//     console.log('viewFunctions: ', viewFunctions)

//     nonce = 0;
//     for (let [sponsor, keys] of Object.entries(keyPairsForSponsors)) {
//         let nftTotalSupply = await nftSeries.view('nft_supply_for_series', {mint_id: nonce});
//         console.log(`nftTotalSupply for ${sponsor}: ${nftTotalSupply}`);
//         nonce += 1;
//     }
// });

test('Force Injecting Fields Attack', async t => {
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;

    console.log("adding to balance");
    await owner.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("5").toString()});
    await ali.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("5").toString()});

    let keyPairs = await generateKeyPairs(10);

    await nftSeries.call(nftSeries, 'create_series', {mint_id: 0, metadata: nftMetadata}, {attachedDeposit: NEAR.parse("0.02").toString()});

    // Creating the legit drop
    await owner.call(keypom, 'create_drop', {
        public_keys: [keyPairs.publicKeys[0]], 
        deposit_per_use: NEAR.parse('20 mN').toString(),
        fc_data: getNEARConFCData(nftSeries),
        config: dropConfig,
    },{gas: LARGE_GAS});

    // Creating the first malicious drop
    await ali.call(keypom, 'create_drop', {
        public_keys: [keyPairs.publicKeys[1]], 
        deposit_per_use: NEAR.parse('20 mN').toString(),
        fc_data: {
            methods: [
                null,
                null,
                [{
                    receiver_id: nftSeries,
                    method_name: "nft_mint",
                    args: JSON.stringify({
                        mint_id: "0",
                        receiver_id: ali.accountId
                    }),
                    attached_deposit: NEAR.parse("0.015").toString(),
                    account_id_field: "receiver_id",
                    drop_id_field: "mint_id"
                }]
            ]
        },
        config: dropConfig,
    },{gas: LARGE_GAS});

    // Second malicious drop
    await ali.call(keypom, 'create_drop', {
        public_keys: [keyPairs.publicKeys[2]], 
        deposit_per_use: NEAR.parse('20 mN').toString(),
        fc_data: {
            methods: [
                null,
                null,
                [{
                    receiver_id: nftSeries,
                    method_name: "nft_mint",
                    args: JSON.stringify({
                        mint_id: "0",
                        receiver_id: ali.accountId
                    }),
                    attached_deposit: NEAR.parse("0.015").toString(),
                }]
            ]
        },
        config: dropConfig,
    },{gas: LARGE_GAS});

    // Third malicious drop
    await ali.call(keypom, 'create_drop', {
        public_keys: [keyPairs.publicKeys[3]], 
        deposit_per_use: NEAR.parse('20 mN').toString(),
        fc_data: {
            methods: [
                null,
                null,
                [{
                    receiver_id: nftSeries,
                    method_name: "nft_mint",
                    args: JSON.stringify({
                        mint_id: "0",
                        receiver_id: ali.accountId
                    }),
                    attached_deposit: NEAR.parse("0.015").toString(),
                    account_id_field: "foobar",
                    drop_id_field: "barfoo"
                }]
            ]
        },
        config: dropConfig,
    },{gas: LARGE_GAS});

    let newKeyPairs = await generateKeyPairs(4);
    for(let i = 0; i < 4; i++) {
        await keypom.setKey(keyPairs.keys[i]);

        await keypom.call(keypom, 'claim', { account_id: bob }, { gas: WALLET_GAS });
        await keypom.call(keypom, 'claim', { account_id: bob }, { gas: WALLET_GAS });
        await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${i}.test.near`, new_public_key : newKeyPairs.publicKeys[i]}, {gas: WALLET_GAS});
    }

    let getSeries = await nftSeries.view('get_series', {});
    console.log('getSeries: ', getSeries)
    
    let supplySeries = await nftSeries.view('get_supply_series', {});
    console.log('supplySeries: ', supplySeries)

    let nftTotalSupply = await nftSeries.view('nft_total_supply', {});
    console.log('nftTotalSupply: ', nftTotalSupply)

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
    });

    console.log('viewFunctions: ', viewFunctions)
    let nftTotalSupplyForSeries = await nftSeries.view('nft_supply_for_series', {mint_id: 0});
    console.log(`nftTotalSupply: ${nftTotalSupplyForSeries}`);
});