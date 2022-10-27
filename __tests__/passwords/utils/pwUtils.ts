import { KeyPair, NEAR, NearAccount, PublicKey } from "near-workspaces";
import { generateKeyPairs, LARGE_GAS } from "../../utils/general";
import { createHash } from "crypto";

export function hash(string: string) {
    let h1 = createHash('sha256').update(string).digest('base64');
    console.log('h1: ', h1)
    let h2 = btoa(string);
    console.log('h2: ', h2)
    return h2
}

export function generateGlobalPasswords(
    pubKeys: string[],
    basePassword: string,
): string[]  {
    let passwords: string[] = [];
    for (var key in pubKeys) {
        passwords.push(hash(hash(basePassword + key)));
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
                let jsonPw = {
                    pw: hash(hash(basePassword + pubKeys[i] + use.toString())),
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