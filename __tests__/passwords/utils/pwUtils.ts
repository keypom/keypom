import { KeyPair, NEAR, NearAccount, PublicKey } from "near-workspaces";
import { generateKeyPairs, LARGE_GAS } from "../../utils/general";
import { createHash } from "crypto";

export function hash(string: string, double=false) {
    if (double) {
        return createHash('sha256').update(Buffer.from(string, 'hex')).digest('hex');
    }

    return createHash('sha256').update(Buffer.from(string)).digest('hex');
}

export function generateGlobalPasswords(
    pubKeys: string[],
    basePassword: string,
): string[]  {
    let passwords: string[] = [];
    for (var key in pubKeys) {
        passwords.push(hash(hash(basePassword + key), true));
    }
    return passwords;
}

export function generateLocalPasswords(
    // All pubKeys
    pubKeys: string[],
    keysWithPws: string[],
    usesWithPws: number[],
    basePassword: string
): Array<Array<{ pw: string; key_use: number } | undefined> | undefined> {
    let passwords: Array<Array<{ pw: string; key_use: number } | undefined> | undefined> = [];
    
    for (var i = 0; i < pubKeys.length; i++) {
        console.log('i: ', i)
        if (keysWithPws.includes(pubKeys[i])) {
            let passwordsPerUse: Array<{ pw: string; key_use: number }> = [];
            for (var use in usesWithPws) {
                console.log('use: ', use)
                console.log("INNER HASH: ", hash(basePassword + pubKeys[i] + use.toString()))
                let jsonPw = {
                    pw: hash(hash(basePassword + pubKeys[i] + use.toString()), true),
                    key_use: parseInt(use)
                }
                console.log('jsonPw: ', jsonPw)
                passwordsPerUse.push(jsonPw);
            }
            passwords.push(passwordsPerUse);
        } else {
            console.log('undefined')
            passwords.push(undefined);
        }
    }

    return passwords;
}