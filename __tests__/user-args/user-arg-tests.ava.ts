import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA } from "../utils/general";

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

test('Fully Claim all ticketing keys', async t => {
    const { keypom, nftSeries, owner, ali, bob } = t.context.accounts;
});