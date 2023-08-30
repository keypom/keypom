import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { claimWithRequiredGas, displayBalances, doesDropExist, functionCall, generateKeyPairs } from "../utils/general";
import { FCData } from "@keypom/core";
import { totalSupply } from "../utils/ft-utils";
const { readFileSync } = require('fs');

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
    rpcPort: string;
  }>;

test.beforeEach(async (t) => {
    console.log(t.title);
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    const rpcPort = (worker as any).config.rpcAddr
    console.log(`rpcPort: `, rpcPort)
    
    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;
    console.log('root: ', root)
    
    const keypomV3 = await root.createSubAccount('keypom');
    console.log('keypomV3: ', keypomV3.accountId)

    // Test users
    const funder = await root.createSubAccount('funder');
    console.log('funder: ', funder.accountId)
    await funder.updateAccount({
        amount: NEAR.parse('10000000 N').toString()
    })
    const nftContract = await root.devDeploy(`./__tests__/ext-wasm/nft-tutorial.wasm`);
    const ftContract1 = await root.createSubAccount('ft_contract_1');
    await ftContract1.deploy(`./__tests__/ext-wasm/ft.wasm`);

    await nftContract.call(nftContract, 'new_default_meta', { owner_id: nftContract });
    await ftContract1.call(ftContract1, 'new_default_meta', { owner_id: ftContract1, total_supply: totalSupply.toString() });

    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: keypomV3.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'storage_deposit', args: {account_id: funder.accountId},attachedDeposit: NEAR.parse("1").toString(), shouldLog: false})
    await functionCall({signer: ftContract1, receiver: ftContract1, methodName: 'ft_transfer', args: {receiver_id: funder.accountId, amount: NEAR.parse("1000").toString()},attachedDeposit: "1", shouldLog: false})

    
    await keypomV3.deploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    console.log('Deployed contracts: ', keypomV3.accountId)
    
    await root.call(root, 'new', {});
    await keypomV3.call(keypomV3, 'new', { root_account: root.accountId, owner_id: keypomV3.accountId, contract_metadata: {version: "3.0.0", link: "hello"} });
    
    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, funder, nftContract, ftContract1, keypomV3 }
    t.context.rpcPort = rpcPort;
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

const TERA_GAS = 1000000000000;

test('All Valid Methods', async t => {
    const { keypomV3, nftContract, funder, ali, bob } = t.context.accounts;

    let method1 = {
        receiver_id: nftContract.accountId,
        method_name: 'nft_mint',
        args: JSON.stringify({
            token_id: '1',
            metadata: {
                title: "foo"
            }
        }),
        attached_deposit: NEAR.parse("1").toString(),
        attached_gas: (20 * TERA_GAS).toString(),
        keypom_args: {
            account_id_field: "receiver_id",
        },
    }

    const fcAsset1 = [method1]
    

    const dropId = "drop-id";

    let {keys, publicKeys} = await generateKeyPairs(1);
    await functionCall({
        signer: funder,
        receiver: keypomV3,
        methodName: 'create_drop',
        args: {
            drop_id: dropId,
            asset_data: [{
                assets: [fcAsset1],
                uses: 3
            }],
            key_data: [
                {
                    public_key: publicKeys[0]
                }
            ],
        },
        attachedDeposit: NEAR.parse("21").toString()
    })

   t.is(await doesDropExist(keypomV3, dropId), true)
});

test('Blacklisted Functions and Receivers', async t => {
    const { keypomV3, nftContract, funder, ali, bob, ftContract1 } = t.context.accounts;

    let bannedMethodNames: string[]= ["nft_transfer", "nft_transfer_call", "nft_approve", "nft_transfer_payout", "ft_transfer", "ft_transfer_call"]
    let bannedMethods: {receiver_id: string, method_name: string, args: string, attached_deposit: string, attached_gas: string}[]=[]
    // 6 Banned methods + 1 directed to Keypom
    for(let i = 0; i < bannedMethodNames.length + 1; i++){
        bannedMethods.push({
            receiver_id: i < 2 ? nftContract.accountId : i < bannedMethodNames.length ? ftContract1.accountId : keypomV3.accountId,
            method_name: i < bannedMethodNames.length ? bannedMethodNames[i] : "create_drop",
            args: JSON.stringify({}),
            attached_deposit: "0",
            attached_gas: (20 * TERA_GAS).toString(),
        })
    }

    const dropId = "drop-id";
    for(let i = 0; i < bannedMethods.length; i++){
        let initialBal = await keypomV3.balance()
        console.log(`Attempting create_drop with ${bannedMethods[i].method_name} on ${bannedMethods[i].receiver_id}`)

        let bannedFcMethodAsset = [bannedMethods[i]]
        let {keys, publicKeys} = await generateKeyPairs(1);
        await functionCall({
            signer: funder,
            receiver: keypomV3,
            methodName: 'create_drop',
            args: {
                drop_id: dropId,
                asset_data: [{
                    assets: [bannedFcMethodAsset],
                    uses: 1
                }],
                key_data: [
                    {
                        public_key: publicKeys[0]
                    }
                ],
            },
            attachedDeposit: NEAR.parse("1").toString(),
            shouldPanic: true
        })
        
        t.is(await doesDropExist(keypomV3, dropId), false)
        let finalBal = await keypomV3.balance()
        t.deepEqual(finalBal.stateStaked, initialBal.stateStaked);
        
    }
});