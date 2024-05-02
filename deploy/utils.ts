import {
  artworkUrls,
  descriptions,
  eventDescriptions,
  eventNames,
  locations,
  questions,
  ticketArtworkUrls,
  ticketTypes,
} from "./dummyData";
import {
  DateAndTimeInfo,
  FunderMetadata,
  QuestionInfo,
  ZombieDropMetadata,
  ZombieReturnedEvent,
} from "./interfaces";
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

// Delay function
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

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
  console.log(
    "Sending transaction... with deposit",
    utils.format.parseNearAmount(deposit),
  );
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

export async function createContracts({
  signerAccount,
  near,
  marketplaceContractId,
  keypomContractId,
}: {
  signerAccount: any;
  near: any;
  marketplaceContractId: string;
  keypomContractId: string;
}) {
  const secretKeys = [
    "ed25519:3SoKRJxQj29Kczj6TiMNNxq3c6S3WcA4MUb6oBEcHMEDyhLWxzJyWxXn69sKt3RKCs8akb5KHkNvjdq4mJLYCYGA",
    "ed25519:36s6Po4JAJVhQdZLmXx8gJh4HP6T4AtMYJb5UgWhiYLntxpTX5piAc2pGExdUYrFJujYi18ZevU1z52CRkL6kpLF",
    "ed25519:3c4EMC5d2DEvXjHxQZN7ef7pj8FDK7B9y4QfuvossLSBwntPorYsqxbyuh4vAqTEr397Dgabxjz1RTe9p4CiKs8C",
    "ed25519:5bx66kw3oPfa3vNXe2hXKcN6M5jqMJyC5WHAx8hFzDG4j68QXu8E4wdkaY1H1SvUzbdkxGpyio5Gxmfx8SrBjrg3",
    "ed25519:5JiAyefRbZsy8Yc4gBMJRJpsTFekxYCtvnZgBJUtQzqmRhX6VVB8ZwAnYcy3rdzquN5WPSrw9fUuwhD612L5RqzA",
    "ed25519:oNpuBdJY4HSGZhi1oSEeUWxErdSTjzLmJdx5erMDrV9hzrGVE8peAjRb2QVY4aP81ctf96YpWe419Fr2wqrc7mG",
    "ed25519:nzf775uk2hBRoZXk41kXMqLRofKK2E5qGi3jUjfQnKZvq22f7qhkzCyhenxuRqQMac4tFgGcSsogmFCmimNWkq5",
    "ed25519:2DwsKo8ZwVotcTtTLLRwx6iHamgxG9qFHo6T5G6udMQMyLNxeqo2oy3Z7UFwFJc5ztdqGCc3b4SidJUDcAhkF7V8",
    "ed25519:3PjnkrRdAmWEfoKBkdSXXfGs6AWB6rTG5Sva9EEJBAnAKnkbNwk5VsXPx43zFmKJJhfHwzgFM76FVGqmZQjh6wWh",
    "ed25519:2nP3KsnqWb96k6HKNXHiyumtGH6pmoBLvVqUAbNBAQFMBzTjho3Nw7Yo5fwDMZFwgPeaEeYMcGCqkmX1eoL8Abw1"
  ]
  const publicKeys: String[] = [];
  for (const secretKey of secretKeys) {
    const keyPair = KeyPair.fromString(secretKey);
    publicKeys.push(keyPair.publicKey.toString());
  }

  await createAccountDeployContract({
    signerAccount,
    newAccountId: keypomContractId,
    amount: "20",
    near,
    wasmPath: "./out/keypom.wasm",
    methodName: "new",
    args: {
      root_account: "testnet",
      owner_id: keypomContractId,
      signing_pks: publicKeys,
      signing_admins: ["minqi.testnet", "benjiman.testnet", "minqianlu.testnet"],
      message: "Keypom is lit!",
    },
    deposit: "0",
    gas: "300000000000000",
  });

  await createAccountDeployContract({
    signerAccount,
    newAccountId: marketplaceContractId,
    amount: "20",
    near,
    wasmPath: "./out/marketplace.wasm",
    methodName: "new",
    args: {
      keypom_contract: keypomContractId,
      owner_id: "minqi.testnet",
      v2_keypom_contract: "v2.keypom.testnet",
    },
    deposit: "0",
    gas: "300000000000000",
  });
}

export async function createAccountDeployContract({
  signerAccount,
  newAccountId,
  amount,
  near,
  wasmPath,
  methodName,
  args,
  deposit = "0",
  gas = "300000000000000",
}: {
  signerAccount: any;
  newAccountId: string;
  amount: string;
  near: any;
  wasmPath: string;
  methodName: string;
  args: any;
  deposit?: string;
  gas?: string;
}) {
  console.log("Creating account: ", newAccountId);
  await createAccount({ signerAccount, newAccountId, amount });
  console.log("Deploying contract: ", newAccountId);
  const accountObj = await near.account(newAccountId);
  await sendTransaction({
    signerAccount: accountObj,
    receiverId: newAccountId,
    methodName,
    args,
    deposit,
    gas,
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
  // const keyPair = KeyPair.fromRandom("ed25519");
  const keyPair = KeyPair.fromString(
    "ed25519:2vQcYHvPqBrzTnAyeWVConoYVRR25dwj2UNqPXkWrU88L47B1FoWZaXXwWtr7hBFBge5pFwTdYzjtrUN8pTKpsxY",
  );
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

export function generateEvents(numEvents = 40) {
  // Assume necessary context (like eventThemes, locations, etc.) is defined elsewhere

  function randomDate(start: Date, end: Date): Date {
    return new Date(
      start.getTime() + Math.random() * (end.getTime() - start.getTime()),
    );
  }

  function formatDateToEpoch(date: Date): number {
    return date.getTime();
  }

  function formatTime(date: Date): string {
    let hours = date.getHours();
    let minutes = date.getMinutes();
    let ampm = hours >= 12 ? "PM" : "AM";
    hours = hours % 12;
    hours = hours ? hours : 12; // the hour '0' should be '12'
    minutes = minutes < 10 ? 0 + minutes : minutes;
    let strTime = hours + ":" + minutes + " " + ampm;
    return strTime;
  }

  function generateDateInfo(): DateAndTimeInfo {
    const startDate = new Date(2023, 0, 1);
    const endDate = new Date(2024, 11, 31);
    const start = randomDate(startDate, endDate);

    if (Math.random() > 0.5) {
      // Single day event
      return {
        startDate: formatDateToEpoch(start),
        startTime: formatTime(start), // Optional
      };
    } else {
      // Multi-day event
      const end = new Date(
        start.getTime() + Math.random() * (endDate.getTime() - start.getTime()),
      );
      end.setDate(end.getDate() + Math.floor(Math.random() * 4) + 1); // 1 to 5 days duration
      return {
        startDate: formatDateToEpoch(start),
        startTime: formatTime(start), // Optional
        endDate: formatDateToEpoch(end),
        endTime: formatTime(end), // Optional
      };
    }
  }

  function generateQuestions() {
    if (Math.random() > 0) {
      // Single day event
      return questions.slice(0, 5);
    } else {
      return undefined;
    }
  }

  let events: ZombieReturnedEvent[] = [];
  for (let i = 0; i < numEvents; i++) {
    const themeIndex = Math.floor(
      Math.random() *
        (eventNames.length <= eventDescriptions.length
          ? eventNames.length
          : eventDescriptions.length),
    );

    const eventName = `${eventNames[themeIndex]}`;
    const eventId = crypto.randomUUID().toString();
    const eventInfo = {
      name: eventName,
      dateCreated: Date.now().toString(),
      id: eventId,
      description: `${eventDescriptions[themeIndex]}`,
      location: `${locations[Math.floor(Math.random() * locations.length)]}`,
      // description: ``,
      // location: ``,
      date: generateDateInfo(),
      artwork: artworkUrls[Math.floor(Math.random() * artworkUrls.length)],
      // artwork: '',
      questions: generateQuestions(),
      nearCheckout: true,
    };

    let tickets: ZombieDropMetadata[] = [];
    const numTickets = Math.floor(Math.random() * 6) + 1;
    for (let j = 0; j < numTickets; j++) {
      const ticketType: string =
        ticketTypes[Math.floor(Math.random() * ticketTypes.length)];
      const ticketInfo = {
        name: `${ticketType} Ticket`,
        eventId,
        description: descriptions[ticketType],
        salesValidThrough: generateDateInfo(),
        passValidThrough: generateDateInfo(),
        price:
          Math.random() > 0.5
            ? `${utils.format.parseNearAmount(
                (Math.floor(Math.random() * 150) + 1).toString(),
              )}`
            : "0", // $25 to $500
        artwork:
          ticketArtworkUrls[
            Math.floor(Math.random() * ticketArtworkUrls.length)
          ],
        maxSupply: Math.floor(Math.random() * 50) + 1,
        dateCreated: new Date().toISOString(),
      };
      tickets.push(ticketInfo);
    }

    events.push({
      eventMeta: eventInfo,
      tickets: tickets,
    });
  }

  return events;
}

export function uint8ArrayToBase64(u8Arr: Uint8Array): string {
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
      modulusLength: 4096,
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
  saltBase64: string,
): Promise<any> {
  // Convert Base64-encoded salt back to Uint8Array
  const salt = Uint8Array.from(atob(saltBase64), (c) => c.charCodeAt(0));

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

export async function exportPublicKeyToBase64(publicKey: any) {
  // Export the key to the SPKI format
  const exportedKey = await crypto.subtle.exportKey("spki", publicKey);

  // Convert the exported key to a Base64 string
  const base64Key = arrayBufferToBase64(exportedKey);

  return base64Key;
}

export function arrayBufferToBase64(buffer: any) {
  let binary = "";
  const bytes = new Uint8Array(buffer);
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

async function base64ToPublicKey(base64Key: string) {
  // Decode the Base64 string to an ArrayBuffer
  const binaryString = atob(base64Key);
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }

  // Import the key from the ArrayBuffer
  const publicKey = await crypto.subtle.importKey(
    "spki",
    bytes.buffer,
    {
      name: "RSA-OAEP",
      hash: { name: "SHA-256" },
    },
    true,
    ["encrypt"],
  );

  return publicKey;
}

export const addTickets = async ({
  signerAccount,
  funderAccountId,
  keypomAccountId,
  marketplaceAccount,
  dropId,
  ticket,
  eventId,
  eventQuestions,
}: {
  signerAccount: any;
  funderAccountId: string;
  keypomAccountId: string;
  marketplaceAccount: any;
  dropId: string;
  ticket: ZombieDropMetadata;
  eventId: string;
  eventQuestions?: QuestionInfo[];
}): Promise<string[]> => {
  const maxSupply = ticket.maxSupply || 100;
  let numTickets = Math.floor(Math.random() * maxSupply) + 1;
  numTickets = Math.min(numTickets, maxSupply);

  let keyData: {
    public_key: string;
    metadata: string;
    key_owner?: string;
  }[] = [];
  let keyPairs: string[] = [];

  const funderInfo = await signerAccount.viewFunction(
    keypomAccountId,
    "get_funder_info",
    { account_id: funderAccountId },
  );

  const funderMeta: FunderMetadata = JSON.parse(funderInfo.metadata);
  // console.log("Funder Metadata: ", funderMeta);
  const eventInfo = funderMeta[eventId];

  let pubKey;
  if (eventInfo.pubKey !== undefined) {
    pubKey = await base64ToPublicKey(eventInfo.pubKey);
    console.log("Public Key: ", pubKey);
  }

  for (let i = 0; i < numTickets; i++) {
    const keyPair = KeyPair.fromRandom("ed25519");
    keyPairs.push(keyPair.toString());
    const publicKey = keyPair.publicKey.toString();
    const questions = eventQuestions || [];

    let answers: { [key: string]: string } = {};
    for (const question of questions) {
      if (question.required || Math.random() > 0.8) {
        answers[question.question] = `${question.question}`;
      }
    }

    let metadata = JSON.stringify({ questions: answers });
    if (pubKey !== undefined) {
      metadata = await encryptWithPublicKey(metadata, pubKey);
      // console.log("Encrypted Metadata: ", metadata);
    }

    keyData.push({
      public_key: publicKey,
      metadata,
    });
  }

  await delay(1000); // Delay to prevent nonce retries exceeded error

  try {
    await sendTransaction({
      signerAccount: marketplaceAccount,
      receiverId: keypomAccountId,
      methodName: "add_keys",
      args: {
        drop_id: dropId,
        key_data: keyData,
      },
      deposit: "5",
      gas: "300000000000000",
    });
    return keyPairs;
  } catch (e) {
    console.log("(Add Tix) ERROR!!!: ", e);
  }
  return [];
};

// async function foo() {
//   // Generate a random key pair
//   const { publicKey, privateKey } = await generateKeyPair();
//
//   // Step 2: Encrypt data using the public key
//   const encryptedData = await encryptWithPublicKey(dataToEncrypt, publicKey);
//   console.log("Encrypted Data:", encryptedData);
//
//   // Step 3: Derive a symmetric key from the password
//   const saltHex = crypto.randomBytes(16).toString("hex");
//   const symmetricKey = await deriveKeyFromPassword(masterKey, saltHex);
//
//   // Step 4: Encrypt the private key using the symmetric key
//   const { encryptedPrivateKeyBase64, ivBase64 } = await encryptPrivateKey(
//     privateKey,
//     symmetricKey,
//   );
//   console.log("Encrypted Private Key:", encryptedPrivateKeyBase64);
//
//   // Simulate storing and later retrieving the encrypted private key and iv
//   const storedEncryptedPrivateKey = encryptedPrivateKeyBase64;
//   const storedIv = ivBase64;
//
//   // Step 5: Decrypt the private key using the symmetric key
//   const decryptedPrivateKey = await decryptPrivateKey(
//     storedEncryptedPrivateKey,
//     storedIv,
//     symmetricKey,
//   );
//
//   // Step 6: Decrypt the encrypted data using the decrypted private key
//   const decryptedData = await decryptWithPrivateKey(
//     encryptedData,
//     decryptedPrivateKey,
//   );
//   console.log("Decrypted Data:", decryptedData);
// }
