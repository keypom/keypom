import * as crypto from "crypto";
import { KeyPair, NEAR, NearAccount } from "near-workspaces";
import { readFileSync } from 'fs';
import { getKeyInformation } from "../utils/general";
import { JsonDrop, JsonKeyInfo } from "../utils/types";
import { BN } from "bn.js";

export interface ListingJson {
    nft_token_id: string,
    nft_approval_id: number,
    nft_owner_id: string,
    nft_contract_id: string,
    price: string,
    currency: string,
    created_at: string,
    current_offer?: OfferJson,
}

export interface OfferJson {
    offerer_id: string,
    amount: string,
    referrer_id?: string,
    referral_cut?: number
}
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

export const hashBuf = (str: string, fromHex = false): Promise<ArrayBuffer> =>
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
const attachedDeposit = NEAR.parse("1").toString();
const extraAllowance = NEAR.parse("0.1").toString();

export const createDropArgs = ({
    pubKeys, 
    nftOwners,
    root, 
    passwords, 
    wasmDirectory
}: {
    pubKeys: string[];
    root: NearAccount;
    nftOwners?: string[];
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
        key_owners: nftOwners,
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

export const sellNFT = async ({
    keypom, 
    mintbase, 
    seller, 
    buyer, 
    sellerKeys, 
    buyerKeys, 
    t, 
    tokenId
}: {
    keypom: NearAccount;
    mintbase: NearAccount;
    seller: NearAccount;
    buyer: NearAccount;
    sellerKeys: { keys: KeyPair[]; publicKeys: string[] };
    buyerKeys: { keys: KeyPair[]; publicKeys: string[] };
    t: any;
    tokenId: string;
}) => {
    // Now with migration out of the way, we can test the new mintbase contract and sell access keys
    let initialAllowance = (await getKeyInformation(keypom, sellerKeys.publicKeys[0])).allowance;
    console.log('initialAllowance: ', initialAllowance)

    await keypom.setKey(sellerKeys.keys[0]);
    let new_mintbase_args = JSON.stringify({
        price: NEAR.parse('1').toString(),
        owner_pub_key: seller == keypom ? sellerKeys.publicKeys[0] : undefined
    })
    await keypom.call(keypom, 'nft_approve', {account_id: mintbase.accountId, msg: new_mintbase_args});
    let listing: ListingJson = await mintbase.view('get_listing', {nft_contract_id: keypom, token_id: tokenId});
    t.assert(listing.nft_token_id === tokenId);
    t.assert(listing.price === NEAR.parse('1').toString());
    t.assert(listing.nft_owner_id === seller.accountId);
    t.assert(listing.nft_contract_id === keypom.accountId);
    t.assert(listing.currency === 'near');

    // After key is put for sale, its allowance should have decremented
    let keyInfo: JsonKeyInfo = await getKeyInformation(keypom, sellerKeys.publicKeys[0]);
    t.assert(new BN(initialAllowance).gt(new BN(keyInfo.allowance)));
    initialAllowance = keyInfo.allowance;

    /// Buyer purchases the key
    await buyer.call(mintbase, 'buy', {nft_contract_id: keypom.accountId, token_id: tokenId, new_pub_key: buyerKeys.publicKeys[0]}, {attachedDeposit: NEAR.parse('1').toString(), gas: '300000000000000'});

    // Now that buyer bought the key, his key should have the same allowance as what seller left off with and should have all remaining uses
    keyInfo = await getKeyInformation(keypom, buyerKeys.publicKeys[0]);
    t.is(keyInfo.owner_id, buyer.accountId);
    t.is(keyInfo.allowance, initialAllowance)
    t.is(keyInfo.remaining_uses, 2);

    try {
        // Seller should now have a simple $NEAR drop with 0.05 $NEAR less than the 1 $NEAR purchase price
        let sellerNewDrop: JsonDrop = await keypom.view('get_drop_information', {key: sellerKeys.publicKeys[0]});
        if (seller == keypom) {
            t.is(sellerNewDrop.deposit_per_use, NEAR.parse('0.95').toString());
            t.is(sellerNewDrop.fc, undefined);
            t.is(sellerNewDrop.ft, undefined);
            t.is(sellerNewDrop.nft, undefined);
            t.assert(sellerNewDrop.simple !== undefined);
        } else {
            t.fail();
        }
    } catch(e) {
        seller == keypom ? t.fail() : t.pass();
    }
}