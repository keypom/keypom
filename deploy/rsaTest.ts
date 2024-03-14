import {
  decryptPrivateKey,
  decryptWithPrivateKey,
  deriveKeyFromPassword,
  encryptPrivateKey,
  encryptWithPublicKey,
  exportPublicKeyToBase64,
  generateKeyPair,
  uint8ArrayToBase64,
} from "./utils";
import * as crypto from "crypto";

function generateRandomString(length: number): string {
    const characters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()";
    let result = "";
    for (let i = 0; i < length; i++) {
        const randomIndex = Math.floor(Math.random() * characters.length);
        result += characters.charAt(randomIndex);
    }
    return result;
}

async function main(){
    const key = await generateKeyPair();
    const message_short = "abc";
    const message_long = generateRandomString(446);
    console.log(message_long)
    console.log("message_long_bytes: ", Buffer.byteLength(message_long, 'utf8'));

    const encrypted_short = await encryptWithPublicKey(message_short, key.publicKey);
    const encrypted_long = await encryptWithPublicKey(message_long, key.publicKey);

    const enc_short_length = Buffer.byteLength(encrypted_short, 'base64');
    const enc_long_length = Buffer.byteLength(encrypted_long, 'base64');

    console.log("short: ", enc_short_length);
    console.log("long: ", enc_long_length);

    console.log(await decryptWithPrivateKey(encrypted_short, key.privateKey));
    console.log(await decryptWithPrivateKey(encrypted_long, key.privateKey));
}

main()