import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, Worker } from "near-workspaces";
import { CONTRACT_METADATA, claimWithRequiredGas, functionCall, generateKeyPairs } from "../utils/general";
import { keypom_args, nftMetadata, nftSeriesMetadata } from "./utils/nft-utils";
import { generatePasswordsForKey, hash } from "./utils/pwUtils";
import { oneGtNear, totalSupply } from "../ft-drops/utils/ft-utils";

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
    const ftContract = await root.devDeploy(`./__tests__/ext-wasm/ft.wasm`);

    // Init the 3 contracts
    await root.call(root, 'new', {});
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    await nftSeries.call(nftSeries, 'new', { owner_id: nftSeries, metadata: nftSeriesMetadata });
    await ftContract.call(ftContract, 'new_default_meta', { owner_id: ftContract, total_supply: totalSupply.toString() });

    // Test users
    const ali = await root.createSubAccount('ali');
    const funder = await root.createSubAccount('funder');
    const minter = await root.createSubAccount('minter');

    // Mint the FTs
    await ftContract.call(ftContract, 'storage_deposit', { account_id: minter.accountId }, { attachedDeposit: NEAR.parse("1").toString() });
    await ftContract.call(ftContract, 'ft_transfer', { receiver_id: minter.accountId, amount: (oneGtNear * BigInt(1000)).toString() }, { attachedDeposit: "1" });
    
    // Add 10k $NEAR to funder's account
    await funder.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })
    await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: "0"});

    // Mint the NFT
    await nftSeries.call(nftSeries, 'create_series', { mint_id: 0, metadata: nftMetadata }, { attachedDeposit: NEAR.parse("1").toString() });
    await nftSeries.call(nftSeries, 'add_approved_minter', { account_id: keypom.accountId });
    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, funder, ali, minter, ftContract, nftSeries };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Single Null Claim', async t => {
    //get Keypopm initial balance
    const { keypom, funder, ali, nftSeries, minter, root } = t.context.accounts;

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
            key_data: [],
            asset_data: [
                {
                    uses: 1,
                    assets: [null],
                    config: {
                        permissions: "claim"
                    },
                },
            ]
        },
        attachedDeposit: NEAR.parse("20").toString()
    })

    let numKeys = 1;
    let useWithPw = 1;
    let {keys, publicKeys} = await generateKeyPairs(numKeys);

    let basePassword = 'mypassword1';
    let key_data: Array<{public_key: string, password_by_use?: Record<number, string>}> = [];
    for (var pk of publicKeys) {
        let password_by_use = generatePasswordsForKey(pk, [useWithPw], basePassword);
        
        key_data.push({
            public_key: pk,
            password_by_use
        })
    }

    await functionCall({
        signer: funder,
        receiver: keypom,
        methodName: 'add_keys',
        args: {
            drop_id: dropId,
            key_data
        },
        attachedDeposit: NEAR.parse("20").toString()
    })

    let keyPk = keys[0].getPublicKey().toString();
    const keyInfo = await functionCall({
        signer: keypom,
        receiver: keypom,
        methodName: 'get_key_information',
        args: {
            key: keyPk
        },
        gas: '100000000000000',
        shouldPanic: false
    })
    console.log('keyInfo: ', keyInfo)

    // await claimWithRequiredGas({
    //     keypom,
    //     keyPair: keys[0],
    //     root,
    //     receiverId: ali.accountId,
    //     password: hash(basePassword + publicKeys[0] + useWithPw.toString())
    // })
});

// test('Double Null Claim', async t => {
//     //get Keypopm initial balance
//     const { keypom, funder, ali, nftSeries, minter, root } = t.context.accounts;

//     //add 20 $NEAR to balance
//     console.log("adding to balance");
//     await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

//     const dropId = "drop-id";

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: 
//                 {
//                     assets: [null],
//                     num_uses: 2,
//             },
//             drop_data: {
//                 config: {
//                     usage: {
//                         permissions: "claim"
//                     }
//                 },
//             }
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     let numKeys = 1;
//     let {keys, publicKeys} = await generateKeyPairs(numKeys);

//     let basePassword = 'mypassword1';
//     //generates an array of hash(hash(basePassword + publicKeys[i])) --> all PWs for all key use
//     let password_by_use = generatePasswordsForKey(publicKeys[0], [1], basePassword);
    
//     // Create an array of size numKeys that's filled with objects

//     let key_data: Array<{public_key: string, password_by_use?: Record<number, string>}> = [];
//     for (var pk of publicKeys) {
//         let password_by_use = generatePasswordsForKey(pk, [2], basePassword);
        
//         key_data.push({
//             public_key: pk,
//             password_by_use
//         })
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'add_keys',
//         args: {
//             drop_id: dropId,
//             key_data
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//         password: hash(basePassword + publicKeys[0] + '2')
//     })
// });

// test('Triple Null Claim', async t => {
//     //get Keypopm initial balance
//     const { keypom, funder, ali, nftSeries, minter, root } = t.context.accounts;

//     //add 20 $NEAR to balance
//     console.log("adding to balance");
//     await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

//     const dropId = "drop-id";

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: 
//                 {
//                     assets: [null],
//                     num_uses: 3,
//                  },
//             drop_data: {
//                 config: {
//                     usage: {
//                         permissions: "claim"
//                     }
//                 },
//             }
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     let numKeys = 1;
//     let {keys, publicKeys} = await generateKeyPairs(numKeys);

//     let basePassword = 'mypassword1';
    
//     // Create an array of size numKeys that's filled with objects

//     let key_data: Array<{public_key: string, password_by_use?: Record<number, string>}> = [];
//     for (var pk of publicKeys) {
//         let password_by_use = generatePasswordsForKey(pk, [3], basePassword);
        
//         key_data.push({
//             public_key: pk,
//             password_by_use
//         })
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'add_keys',
//         args: {
//             drop_id: dropId,
//             key_data
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//         password: hash(basePassword + publicKeys[0] + '3')
//     })
// });

// test('Null + NFT Claim', async t => {
//     //get Keypopm initial balance
//     const { keypom, funder, ali, nftSeries, minter, root } = t.context.accounts;

//     //add 20 $NEAR to balance
//     console.log("adding to balance");
//     await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

//     const dropId = "0";

//     let mintMethod = {
//         methods: [
//             {
//                 receiver_id: nftSeries.accountId,
//                 method_name: 'nft_mint',
//                 args: "",
//                 attached_deposit: NEAR.parse("0.0081").toString(),
//                 attached_gas: (8 * 1e12).toString(),
//                 keypom_args: {
//                     account_id_field: "receiver_id",
//                     drop_id_field: "mint_id",
//                 },
//             }
//         ]
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: [
//                 {
//                     assets: [null],
//                     config: {
//                         usage: {
//                             permissions: "claim"
//                         }
//                     },
//                 },
//                 {
//                     assets: [mintMethod],
//                 },
//             ]
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     let numKeys = 1;
//     let {keys, publicKeys} = await generateKeyPairs(numKeys);

//     let basePassword = 'mypassword1';
//     let key_data: Array<{public_key: string, password_by_use?: Record<number, string>}> = [];
//     for (var pk of publicKeys) {
//         let password_by_use = generatePasswordsForKey(pk, [1], basePassword);
        
//         key_data.push({
//             public_key: pk,
//             password_by_use
//         })
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'add_keys',
//         args: {
//             drop_id: dropId,
//             key_data
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//         password: hash(basePassword + publicKeys[0] + '1')
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         useImplicitAccount: true
//     })
// });

// test('2x Null + NFT Claim', async t => {
//     //get Keypopm initial balance
//     const { keypom, funder, ali, nftSeries, minter, root } = t.context.accounts;

//     //add 20 $NEAR to balance
//     console.log("adding to balance");
//     await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

//     const dropId = "0";

//     let mintMethod = {
//         methods: [
//             {
//                 receiver_id: nftSeries.accountId,
//                 method_name: 'nft_mint',
//                 args: "",
//                 attached_deposit: NEAR.parse("0.0081").toString(),
//                 attached_gas: (8 * 1e12).toString(),
//                 keypom_args: {
//                     account_id_field: "receiver_id",
//                     drop_id_field: "mint_id",
//                 },
//             }
//         ]
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: [
//                 {
//                     assets: [null],
//                     config: {
//                         usage: {
//                             permissions: "claim"
//                         }
//                     },
//                 },
//                 {
//                     assets: [null],
//                     config: {
//                         usage: {
//                             permissions: "claim"
//                         }
//                     },
//                 },
//                 {
//                     assets: [mintMethod],
//                 },
//             ]
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     let numKeys = 1;
//     let {keys, publicKeys} = await generateKeyPairs(numKeys);

//     let basePassword = 'mypassword1';
//     let key_data: Array<{public_key: string, password_by_use?: Record<number, string>}> = [];
//     for (var pk of publicKeys) {
//         let password_by_use = generatePasswordsForKey(pk, [2], basePassword);
        
//         key_data.push({
//             public_key: pk,
//             password_by_use
//         })
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'add_keys',
//         args: {
//             drop_id: dropId,
//             key_data
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//         password: hash(basePassword + publicKeys[0] + '2')
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         useImplicitAccount: true
//     })
// });

// test('Null + FT Claim', async t => {
//     //get Keypopm initial balance
//     const { keypom, funder, ali, ftContract, nftSeries, minter, root } = t.context.accounts;

//     //add 20 $NEAR to balance
//     console.log("adding to balance");
//     await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

//     const dropId = "0";
//     const ftContractData = {
//         ft_contract_id: ftContract.accountId,
//         registration_cost: NEAR.parse("0.0125").toString(),
//         ft_amount: NEAR.parse("1").toString()
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: [
//                 {
//                     assets: [null],
//                     config: {
//                         usage: {
//                             permissions: "claim"
//                         }
//                     },
//                 },
//                 {
//                     assets: [ftContractData],
//                 },
//             ]
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     let numKeys = 1;
//     let useWithPw = 1;
//     let {keys, publicKeys} = await generateKeyPairs(numKeys);

//     let basePassword = 'mypassword1';
//     let key_data: Array<{public_key: string, password_by_use?: Record<number, string>}> = [];
//     for (var pk of publicKeys) {
//         let password_by_use = generatePasswordsForKey(pk, [useWithPw], basePassword);
        
//         key_data.push({
//             public_key: pk,
//             password_by_use
//         })
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'add_keys',
//         args: {
//             drop_id: dropId,
//             key_data
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//         password: hash(basePassword + publicKeys[0] + useWithPw.toString())
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         useImplicitAccount: true
//     })
// });

// test('2x Null + FT Claim', async t => {
//     //get Keypopm initial balance
//     const { keypom, funder, ali, ftContract, nftSeries, minter, root } = t.context.accounts;

//     //add 20 $NEAR to balance
//     console.log("adding to balance");
//     await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("20").toString()});

//     const dropId = "0";
//     const ftContractData = {
//         ft_contract_id: ftContract.accountId,
//         registration_cost: NEAR.parse("0.0125").toString(),
//         ft_amount: NEAR.parse("1").toString()
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'create_drop',
//         args: {
//             drop_id: dropId,
//             asset_data: [
//                 {
//                     assets: [null],
//                     config: {
//                         usage: {
//                             permissions: "claim"
//                         }
//                     },
//                 },
//                 {
//                     assets: [null],
//                     config: {
//                         usage: {
//                             permissions: "claim"
//                         }
//                     },
//                 },
//                 {
//                     assets: [ftContractData],
//                 },
//             ]
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     let numKeys = 1;
//     let useWithPw = 2;
//     let {keys, publicKeys} = await generateKeyPairs(numKeys);

//     let basePassword = 'mypassword1';
//     let key_data: Array<{public_key: string, password_by_use?: Record<number, string>}> = [];
//     for (var pk of publicKeys) {
//         let password_by_use = generatePasswordsForKey(pk, [useWithPw], basePassword);
        
//         key_data.push({
//             public_key: pk,
//             password_by_use
//         })
//     }

//     await functionCall({
//         signer: funder,
//         receiver: keypom,
//         methodName: 'add_keys',
//         args: {
//             drop_id: dropId,
//             key_data
//         },
//         attachedDeposit: NEAR.parse("20").toString()
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         useImplicitAccount: true
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         receiverId: ali.accountId,
//         password: hash(basePassword + publicKeys[0] + useWithPw.toString())
//     })

//     await claimWithRequiredGas({
//         keypom,
//         keyPair: keys[0],
//         root,
//         useImplicitAccount: true
//     })
// });