import anyTest, { TestFn } from "ava";
import { NEAR, NearAccount, tGas, Worker } from "near-workspaces";
import { CONTRACT_METADATA, generateKeyPairs, getDropInformation, getKeyInformation, getKeySupplyForDrop, LARGE_GAS, queryAllViewFunctions, WALLET_GAS } from "../utils/general";
import { DropConfig, JsonKeyInfo, NFTTokenObject, SimpleData, TokenMetadata } from "../utils/types";
import { BN } from "bn.js";
import { readFileSync } from 'fs';
import * as crypto from "crypto";

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
  }>;

  test.beforeEach(async (t) => {
    console.log("Starting test");
    // Init the worker and start a Sandbox server
    const worker = await Worker.init();

    // Prepare sandbox for tests, create accounts, deploy contracts, etc.
    const root = worker.rootAccount;

    // Deploy all 2 contracts
    const keypom = await root.devDeploy(`./out/keypom.wasm`);
    await root.deploy(`./__tests__/ext-wasm/linkdrop.wasm`);
    console.log("Deployed contracts");
    
    // Init empty/default linkdrop contract
    await root.call(root, 'new', {});
    //init new keypom contract and setting keypom as the owner. 
    await keypom.call(keypom, 'new', { root_account: 'test.near', owner_id: keypom, contract_metadata: CONTRACT_METADATA });
    
    console.log("Initialized contracts");
    // Test users
    const funder = await root.createSubAccount('funder');
    const bob = await root.createSubAccount('bob');

    // Add 10k $NEAR to owner's account
    await funder.updateAccount({
        amount: NEAR.parse('10000 N').toString()
    })

    // Save state for test runs
    t.context.worker = worker;
    t.context.accounts = { root, keypom, funder, bob };
});

// If the environment is reused, use test.after to replace test.afterEach
test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

// helpers for keypom account contract args
const PARAM_START = "|kP|";
const PARAM_STOP = "|kS|";
export const wrapTxnParamsForTrial = (params: Record<string, string | string[]>) => {
    let newParams: Record<string, string> = {};
    // Loop through each key value in params
    Object.entries(params).forEach(([k, v]) => {
        // If the value is an array, join it with a comma
        if (Array.isArray(v)) v = v.join();
        // Add the key and value to the newParams object
        newParams[PARAM_START.concat(k)] = v.concat(PARAM_STOP);
    });
    return newParams;
};

export interface PasswordPerUse {
    /** The password for this given use */
    pw: string;
    /** Which use does the password belong to? These uses are *NOT* zero-indexed so the first use corresponds to `1` not `0`. */
    key_use: number;
}

const hashBuf = (str: string, fromHex = false): Promise<ArrayBuffer> =>
    // return a promise that resolves to a buffer
    new Promise((resolve, reject) => {
        // create a hash object
        const hash = crypto.createHash('sha256');
        // if the input is hex, convert it to a buffer
        const buf = fromHex ? Buffer.from(str, 'hex') : Buffer.from(str);
        // hash the buffer
        hash.update(buf);
        // resolve the promise with the hash
        resolve(hash.digest());
    }); 

export async function generatePerUsePasswords({
    publicKeys,
    uses,
    basePassword,
}: {
    publicKeys: string[];
    uses: number[];
    basePassword: string;
}): Promise<Array<Array<PasswordPerUse>>> {
    const passwords: Array<Array<PasswordPerUse>> = [];

    // Loop through each pubKey to generate either the passwords
    for (let i = 0; i < publicKeys.length; i++) {
        // For each public key, we need to generate a password for each use
        const passwordsPerUse: Array<PasswordPerUse> = [];
        for (let j = 0; j < uses.length; j++) {
            // First inner hash takes in utf8 and returns hash
            const innerHashBuff = await hashBuf(
                basePassword + publicKeys[i] + uses[j].toString()
            );
            const innerHash = Buffer.from(innerHashBuff).toString('hex');

            // Outer hash takes in hex and returns hex
            const outerHashBuff = await hashBuf(innerHash, true);
            const outerHash = Buffer.from(outerHashBuff).toString('hex');

            const jsonPw = {
                pw: outerHash,
                key_use: uses[j],
            };
            passwordsPerUse.push(jsonPw);
        }
        passwords.push(passwordsPerUse);
    }

    return passwords;
}

test('Base Costs', async t => {
    const { keypom, funder, bob, root } = t.context.accounts;

    const actualContracts = Array(10).fill(keypom.accountId);
    const actualAmounts = Array(10).fill(NEAR.parse("100").toString());
    const actualMethods = Array(10).fill('create_account_and_claim');

    const attachedDeposit = NEAR.parse("30").toString();
    const extraAllowance = NEAR.parse("0.1").toString();
    const basePassword = "MASTER_KEY";

    const {publicKeys} = await generateKeyPairs(50);
    const wasmDirectory = `${require('path').resolve(__dirname, '..')}/ext-wasm/trial-accounts.wasm`
    const dropId = Date.now().toString();
    const createDropArgs = {
        drop_id: dropId,
        public_keys: [],
        deposit_per_use: "0",
        config: {
            uses_per_key: 2,
            nft_key_behaviour: {
                nft_metadata: {
                    title: "NEARCON VIP Ticket",
                    media: 'bafkreib2l2xlbty5uihxgcknqxs5uiinwjx3nif7tk7s32yxcmlr2lbm5i'
                }
            }
        },
        metadata: JSON.stringify({ticketType: "VIP"}),
        required_gas: "150000000000000",
        extra_key_allowance: extraAllowance,
        fc: {
            methods: [
                null,
                [
                    {
                        receiver_id: root.accountId,
                        method_name: "create_account_advanced",
                        attached_deposit: attachedDeposit,
                        args: JSON.stringify({
                            new_account_id: "INSERT_NEW_ACCOUNT",
                            options: {
                                contract_bytes: [...readFileSync(wasmDirectory)],
                                limited_access_keys: [
                                    {
                                        public_key: "INSERT_TRIAL_PUBLIC_KEY",
                                        allowance: "0",
                                        receiver_id: "INSERT_NEW_ACCOUNT",
                                        method_names:
                                            "execute,create_account_and_claim",
                                    },
                                ],
                            },
                        }),
                        user_args_rule: "UserPreferred",
                    },
                    {
                        receiver_id: "",
                        method_name: "setup",
                        attached_deposit: "0",
                        args: JSON.stringify(
                            wrapTxnParamsForTrial({
                                contracts: actualContracts,
                                amounts: actualAmounts,
                                methods: actualMethods,
                                funder: "",
                                repay: "0",
                                floor: "0",
                            })
                        ),
                        receiver_to_claimer: true,
                    },
                ],
            ],
        },
    };

    //add 2NEAR to ali's keypom balance
    await funder.call(keypom, 'add_to_balance', {}, {attachedDeposit: NEAR.parse("5000").toString()});
    
    //create a drop with Ali, doesn't front any cost. 
    await funder.call(keypom, 'create_drop', createDropArgs);

    let passwords: Array<Array<PasswordPerUse>> = await generatePerUsePasswords({
        publicKeys: publicKeys,
        basePassword,
        uses: [1]
    });

    await funder.call(keypom, 'add_keys', {drop_id: dropId, public_keys: publicKeys, passwords_per_use: passwords, extra_key_allowance: extraAllowance,}, {gas: '300000000000000'});
});