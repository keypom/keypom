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
    keysWithPws: string[],
    basePassword: string,
): Array<string | undefined>  {
    let passwords: Array<string | undefined> = [];
    for (var i = 0; i < pubKeys.length; i++) {
        if (keysWithPws.includes(pubKeys[i])) {
            passwords.push(hash(hash(basePassword + pubKeys[i]), true));
        } else {
            passwords.push(undefined);
        }
    }
    return passwords;
}

export function generateLocalPasswords(
    // All pubKeys
    pubKeys: string[],
    // Keys with passwords
    keysWithPws: { [key: string]: number[] },
    basePassword: string
): Array<Array<{ pw: string; key_use: number } | undefined> | undefined> {
    let passwords: Array<Array<{ pw: string; key_use: number } | undefined> | undefined> = [];
    
    // Loop through each pubKey to generate either the password or null
    for (var i = 0; i < pubKeys.length; i++) {
        // If the key has a password
        if (Object.keys(keysWithPws).includes(pubKeys[i])) {
            let passwordsPerUse: Array<{ pw: string; key_use: number }> = [];
            // Key has passwords per use so we should add all of them
            let keyUses = keysWithPws[pubKeys[i]];
            for (var j = 0; j < keyUses.length; j++) {
                let jsonPw = {
                    pw: hash(hash(basePassword + pubKeys[i] + keyUses[j].toString()), true),
                    key_use: keyUses[j]
                }
                passwordsPerUse.push(jsonPw);
            }
            passwords.push(passwordsPerUse);

        // Key has no password so we push undefined
        } else {
            passwords.push(undefined);
        }
    }

    return passwords;
}