import { createHash } from "crypto";

export function hash(string: string, double=false) {
    if (double) {
        return createHash('sha256').update(Buffer.from(string, 'hex')).digest('hex');
    }

    return createHash('sha256').update(Buffer.from(string)).digest('hex');
}

//generate 
export function generatePasswordsForKey(
    pubKey: string,
    usesWithPassword: number[],
    basePassword: string
): Record<number, string> {
    let passwords: Record<number, string> = {}; 

    // Loop through usesWithPassword
    for (var use of usesWithPassword) {
        passwords[use] = hash(hash(basePassword + pubKey + use.toString()), true);
    }

    return passwords;
}

export function generatePasswordsForClaim(
    pubKey: string,
    use: number,
    basePassword: string
): string {
    let pw: string = hash(basePassword + pubKey + use.toString());
    return pw;
}