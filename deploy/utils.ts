import {
  artworkUrls,
  descriptions,
  eventThemes,
  locations,
  questions,
  ticketArtworkUrls,
  ticketTypes,
} from "./dummyData";
import { DropMetadata, Event } from "./interfaces";
import * as crypto from "crypto";

const {
  KeyPair,
  connect,
  utils,
  InMemorySigner,
  transactions,
  keyStores,
} = require("near-api-js");
const fs = require("fs");
const path = require("path");
const homedir = require("os").homedir();

const CREDENTIALS_DIR = ".near-credentials";
const credentialsPath = path.join(homedir, CREDENTIALS_DIR);
const keyStore = new keyStores.UnencryptedFileSystemKeyStore(credentialsPath);

const config = {
  keyStore,
  networkId: "testnet",
  nodeUrl: "https://rpc.testnet.near.org",
};

export async function initNear() {
  const near = await connect({ ...config, keyStore });
  return near;
}

export async function sendTransaction({
  signerAccount,
  receiverId,
  methodName,
  args,
  deposit,
  gas,
  wasmPath = undefined,
}: {
  signerAccount: any;
  receiverId: string;
  methodName: string;
  args: any;
  deposit: string;
  gas: string;
  wasmPath?: string;
}) {
  const result = await signerAccount.signAndSendTransaction({
    receiverId: receiverId,
    actions: [
      ...(wasmPath
        ? [transactions.deployContract(fs.readFileSync(wasmPath))]
        : []),
      transactions.functionCall(
        methodName,
        Buffer.from(JSON.stringify(args)),
        gas,
        utils.format.parseNearAmount(deposit),
      ),
    ],
  });

  console.log(result);
}

export async function deployEventContract({
  signerAccount,
  newAccountId,
  amount,
  near,
  wasmPath,
}: {
  signerAccount: any;
  newAccountId: string;
  amount: string;
  near: any;
  wasmPath: string;
}) {
  console.log("Creating account: ", newAccountId);
  await createAccount({ signerAccount, newAccountId, amount });
  console.log("Deploying contract: ", newAccountId);
  const keyPair = KeyPair.fromRandom("ed25519");
  const publicKey = keyPair.publicKey.toString();
  const keypomAccount = await near.account(newAccountId);
  await sendTransaction({
    signerAccount: keypomAccount,
    receiverId: newAccountId,
    methodName: "new",
    args: {
      root_account: "testnet",
      owner_id: newAccountId,
      signing_pk: publicKey,
      signing_sk: keyPair.secretKey,
      message: "Keypom is lit!",
    },
    deposit: "0",
    gas: "300000000000000",
    wasmPath,
  });
  console.log("Deployed.");
}

export async function createAccount({
  signerAccount,
  newAccountId,
  amount,
}: {
  signerAccount: any;
  newAccountId: string;
  amount: string;
}) {
  const keyPair = KeyPair.fromRandom("ed25519");
  const publicKey = keyPair.publicKey.toString();
  await keyStore.setKey(config.networkId, newAccountId, keyPair);

  return await signerAccount.functionCall({
    contractId: "testnet",
    methodName: "create_account",
    args: {
      new_account_id: newAccountId,
      new_public_key: publicKey,
    },
    gas: "300000000000000",
    attachedDeposit: utils.format.parseNearAmount(amount),
  });
}

export function generateEvents(numEvents = 50) {
  // Helper functions
  function randomDate(start: Date, end: Date) {
    return new Date(
      start.getTime() + Math.random() * (end.getTime() - start.getTime()),
    )
      .toISOString()
      .split("T")[0];
  }

  function formatDate(date: Date) {
    return date.toISOString().split("T")[0];
  }

  function generateEventDate() {
    const startDate = new Date(2023, 0, 1);
    const endDate = new Date(2024, 11, 31);
    if (Math.random() > 0.5) {
      // Single day event
      return { date: randomDate(startDate, endDate) };
    } else {
      // Multi-day event
      const start = new Date(randomDate(startDate, endDate));
      const end = new Date(start);
      end.setDate(end.getDate() + Math.floor(Math.random() * 4) + 1); // 1 to 5 days duration
      return { date: { from: formatDate(start), to: formatDate(end) } };
    }
  }

  function generateQuestions() {
    if (Math.random() > 0.5) {
      // Single day event
      return questions.slice(
        0,
        Math.floor(Math.random() * questions.length) + 1,
      );
    } else {
      return undefined;
    }
  }

  let events: Event[] = [];
  for (let i = 0; i < numEvents; i++) {
    const themeIndex = Math.floor(Math.random() * eventThemes.length);
    const eventName = `${eventThemes[themeIndex]} ${
      ["Festival", "Conference", "Exhibition", "Carnival", "Workshop"][
        Math.floor(Math.random() * 5)
      ]
    }`;
    const eventId = crypto.randomUUID().toString();
    const eventDate = generateEventDate();
    const eventInfo = {
      name: eventName,
      id: eventId,
      description: `A unique ${eventThemes[
        themeIndex
      ].toLowerCase()} experience bringing together the best in the field.`,
      location: `${locations[Math.floor(Math.random() * locations.length)]}`,
      date: eventDate,
      artwork: artworkUrls[Math.floor(Math.random() * artworkUrls.length)],
      questions: generateQuestions(),
    };

    let tickets: DropMetadata[] = [];
    const numTickets = Math.floor(Math.random() * 5) + 1; // 1 to 5 tickets
    for (let j = 0; j < numTickets; j++) {
      const ticketType: string =
        ticketTypes[Math.floor(Math.random() * ticketTypes.length)];
      const ticketInfo = {
        name: `${ticketType} Ticket`,
        eventId,
        description: descriptions[ticketType],
        salesValidThrough: randomDate(
          new Date(2024, 0, 1),
          new Date(2024, 11, 31),
        ),
        passValidThrough: randomDate(
          new Date(2024, 0, 1),
          new Date(2024, 11, 31),
        ),
        price: `${utils.format.parseNearAmount(
          (Math.floor(Math.random() * 451) + 25).toString(),
        )}`, // $25 to $500
        artwork:
          ticketArtworkUrls[
            Math.floor(Math.random() * ticketArtworkUrls.length)
          ],
        maxSupply:
          Math.random() > 0.5
            ? Math.floor(Math.random() * 1000) + 1
            : undefined, // 1 to 100 tickets
      };
      tickets.push({
        dateCreated: new Date().toISOString(),
        dropName: `${ticketType} Ticket for ${eventName}`,
        ticketInfo: ticketInfo,
        eventInfo: j === 0 ? eventInfo : undefined, // Include event info only in the first ticket
      });
    }

    events.push({
      eventInfo: eventInfo,
      tickets: tickets,
    });
  }

  return events;
}

function uint8ArrayToBase64(u8Arr: Uint8Array): string {
  const string = u8Arr.reduce(
    (data, byte) => data + String.fromCharCode(byte),
    "",
  );
  return btoa(string);
}

export async function generateKeyPair(): Promise<{
  privateKey: any;
  publicKey: any;
}> {
  return await crypto.subtle.generateKey(
    {
      name: "RSA-OAEP",
      modulusLength: 2048,
      publicExponent: new Uint8Array([1, 0, 1]),
      hash: { name: "SHA-256" },
    },
    true,
    ["encrypt", "decrypt"],
  );
}

export async function encryptWithPublicKey(
  data: string,
  publicKey: any,
): Promise<string> {
  const encoded = new TextEncoder().encode(data);
  const encrypted = await crypto.subtle.encrypt(
    {
      name: "RSA-OAEP",
    },
    publicKey,
    encoded,
  );

  return uint8ArrayToBase64(new Uint8Array(encrypted));
}

export async function deriveKeyFromPassword(
  password: string,
  saltHex: string,
): Promise<any> {
  // Function to convert hex string to Uint8Array
  function hexStringToUint8Array(hexString: string): Uint8Array {
    const length = hexString.length / 2;
    const uint8Array = new Uint8Array(length);
    for (let i = 0; i < length; i++) {
      uint8Array[i] = parseInt(hexString.substring(i * 2, i * 2 + 2), 16);
    }
    return uint8Array;
  }

  // Convert hex string salt to Uint8Array
  const salt = hexStringToUint8Array(saltHex);

  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(password),
    { name: "PBKDF2" },
    false,
    ["deriveKey"],
  );

  return crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: salt,
      iterations: 100000,
      hash: "SHA-256",
    },
    keyMaterial,
    { name: "AES-GCM", length: 256 },
    true,
    ["encrypt", "decrypt"],
  );
}

export async function encryptPrivateKey(
  privateKey: any,
  symmetricKey: any,
): Promise<{ encryptedPrivateKeyBase64: string; ivBase64: string }> {
  const exportedPrivateKey = await crypto.subtle.exportKey("pkcs8", privateKey);

  const iv = crypto.getRandomValues(new Uint8Array(12));
  const encryptedPrivateKey = await crypto.subtle.encrypt(
    {
      name: "AES-GCM",
      iv: iv,
    },
    symmetricKey,
    exportedPrivateKey,
  );

  const encryptedBase64 = uint8ArrayToBase64(
    new Uint8Array(encryptedPrivateKey),
  );
  const ivBase64 = uint8ArrayToBase64(iv);

  return { encryptedPrivateKeyBase64: encryptedBase64, ivBase64 };
}

export async function decryptPrivateKey(
  encryptedPrivateKeyBase64: string,
  ivBase64: string,
  symmetricKey: any,
): Promise<any> {
  const encryptedPrivateKey = Uint8Array.from(
    atob(encryptedPrivateKeyBase64),
    (c) => c.charCodeAt(0),
  );
  const iv = Uint8Array.from(atob(ivBase64), (c) => c.charCodeAt(0));

  const decryptedPrivateKeyBuffer = await crypto.subtle.decrypt(
    {
      name: "AES-GCM",
      iv: iv,
    },
    symmetricKey,
    encryptedPrivateKey,
  );

  return crypto.subtle.importKey(
    "pkcs8",
    decryptedPrivateKeyBuffer,
    {
      name: "RSA-OAEP",
      hash: { name: "SHA-256" },
    },
    true,
    ["decrypt"],
  );
}

export async function decryptWithPrivateKey(
  encryptedData: string,
  privateKey: any,
): Promise<string> {
  const encryptedDataArrayBuffer = Uint8Array.from(atob(encryptedData), (c) =>
    c.charCodeAt(0),
  ).buffer;

  const decrypted = await crypto.subtle.decrypt(
    {
      name: "RSA-OAEP",
    },
    privateKey,
    encryptedDataArrayBuffer,
  );

  return new TextDecoder().decode(decrypted);
}

export async function generateEncryptedKey(message, publicKey) {}
