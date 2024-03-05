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
