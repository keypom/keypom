import * as crypto from "crypto";
import { NEAR, NearAccount } from "near-workspaces";
import { readFileSync } from 'fs';

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

const actualContracts = Array(10).fill('nft-v2.keypom.near');
const actualAmounts = Array(10).fill(NEAR.parse("100").toString());
const actualMethods = Array(10).fill('create_account_and_claim');
const attachedDeposit = NEAR.parse("25").toString();
const extraAllowance = NEAR.parse("0.1").toString();

export const createDropArgs = ({
    pubKeys, 
    root, 
    passwords, 
    wasmDirectory
}: {
    pubKeys: string[];
    root: NearAccount;
    passwords: Array<Array<PasswordPerUse>>,
    wasmDirectory: string;
}) => {
    return {
        public_keys: pubKeys,
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
        passwords_per_use: passwords,
        metadata: JSON.stringify({ticketType: "standard"}),
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
    }
};