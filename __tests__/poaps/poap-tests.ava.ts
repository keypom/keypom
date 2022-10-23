import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { pagodaDistroPoapsSmall } from "./utils/distro";
import { createDistro, nftSeriesMetadata } from "./utils/nearconUtils";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    keypomInitialBalance: NEAR;
    keypomInitialStateStaked: NEAR;
}>;

const ticketDistribution = pagodaDistroPoapsSmall;
const numTix = Object.values(ticketDistribution).reduce((a, b) => a + b[0], 0);
const numSponsors = Object.keys(ticketDistribution).length;

test.beforeEach(async (t) => {
    // Comment this if you want to see console logs
    //console.log = function() {}

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
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
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

test('Fully Claim all Pagoda POAPs', async t => {
    // Comment this if you want to see console logs
    //console.log = function() {}
    
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
    let ownerBal = "500";
    let depositPerUse = NEAR.parse('200 mN').toString();
    let keyPairsForSponsors = await createDistro(ticketDistribution, owner, keypom, nftSeries, ownerBal, depositPerUse);
    console.log('keyPairsForSponsors: ', keyPairsForSponsors)

    let { keys: keys2, publicKeys: pks2 } = await generateKeyPairs(1);

    let curBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('curBal before: ', curBal)

    let nonce = 0;
    // Iterate through each sponsor in the keyPairsForSponsors object
    for (let [sponsor, keys] of Object.entries(keyPairsForSponsors)) {
        console.log(`Claiming ${keys.length} keys for ${sponsor}`);

        for(let i = 0; i < keys.length; i++) {
            await keypom.setKey(keys[i]);
            await keypom.updateAccessKey(
                keys[i],  // public key
                {
                    nonce: 0,
                    permission: 'FullAccess'
                }
            )

            await keypom.call(keypom, 'create_account_and_claim', {new_account_id: `${nonce}-${i}.test.near`, new_public_key : pks2[0]}, {gas: WALLET_GAS});
        }
        nonce += 1;
    }
    
    curBal = await keypom.view('get_user_balance', {account_id: owner});
    console.log('curBal after: ', curBal)

    let keypomInfo = await keypom.balance();
    console.log('keypom available: ', keypomInfo.available.toString())
    console.log('keypom staked: ', keypomInfo.staked.toString())
    console.log('keypom stateStaked: ', keypomInfo.stateStaked.toString())
    console.log('keypom total: ', keypomInfo.total.toString())

    let nftSeriesInfo = await nftSeries.balance();
    console.log('nft Series available: ', nftSeriesInfo.available.toString())
    console.log('nft Series staked: ', nftSeriesInfo.staked.toString())
    console.log('nft Series stateStaked: ', nftSeriesInfo.stateStaked.toString())
    console.log('nft Series total: ', nftSeriesInfo.total.toString())

    let getSeries = await nftSeries.view('get_series', {});
    console.log('getSeries: ', getSeries)
    
    let supplySeries = await nftSeries.view('get_supply_series', {});
    console.log('supplySeries: ', supplySeries)
    t.is(supplySeries, numSponsors)

    let nftTotalSupply = await nftSeries.view('nft_total_supply', {});
    console.log('nftTotalSupply: ', nftTotalSupply)
    t.is(nftTotalSupply, numTix.toString())

    let viewFunctions = await queryAllViewFunctions({
        contract: keypom, 
        account_id: owner.accountId,
    });
    console.log('viewFunctions: ', viewFunctions)
    t.is(viewFunctions.dropSupplyForOwner, numSponsors)
    t.is(viewFunctions.keyTotalSupply, 0)

    nonce = 0;
    for (let [sponsor, keys] of Object.entries(keyPairsForSponsors)) {
        let nftTotalSupply = await nftSeries.view('nft_supply_for_series', {mint_id: nonce});
        console.log(`nftTotalSupply for ${sponsor}: ${nftTotalSupply}`);
        nonce += 1;
    }
});