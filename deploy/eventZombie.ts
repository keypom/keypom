import { EventInfo, DropMetadata } from "./interfaces";
import {
  createAccount,
  deployEventContract,
  generateEvents,
  initNear,
  sendTransaction,
} from "./utils";
const { KeyPair } = require("near-api-js");

// Delay function
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const fs = require("fs");
const path = require("path");
let allKeyData: { [key: string]: string[] } = {};

const createTicket = async ({
  signerAccount,
  keypomAccountId,
  marketplaceAccount,
  marketplaceAccountId,
  dropId,
  ticket,
  event,
  nonce,
}: {
  signerAccount: any;
  keypomAccountId: string;
  marketplaceAccount: any;
  marketplaceAccountId: string;
  dropId: string;
  ticket: DropMetadata;
  event: EventInfo;
  nonce: number;
}) => {
  console.log(ticket);
  // Create a new drop
  try {
    await sendTransaction({
      signerAccount,
      receiverId: keypomAccountId,
      methodName: "create_drop",
      args: {
        drop_id: dropId,
        drop_config: {
          metadata: JSON.stringify(ticket),
          add_key_allowlist: [marketplaceAccountId],
          transfer_key_allowlist: [marketplaceAccountId],
        },
        key_data: [],
        asset_data: [
          {
            uses: 2,
            assets: [null],
            config: {
              permissions: "claim",
            },
          },
        ],
      },
      deposit: "1",
      gas: "300000000000000",
    });
    console.log(
      "Deployed Ticket: ",
      ticket.ticketInfo.name,
      " with Drop ID: ",
      dropId,
    );
  } catch (e) {
    console.log("(Create Tix) ERROR!!!: ", e);
  }

  let numTickets = Math.floor(Math.random() * 25) + 1; // Number of tickets to mint
  const maxSupply = ticket.ticketInfo.maxSupply || 100;
  numTickets = Math.min(numTickets, maxSupply); // Ensure we don't mint more than the max supply

  let keyData: {
    public_key: string;
    metadata: string;
    key_owner?: string;
  }[] = [];
  let keyPairs: string[] = [];
  for (let i = 0; i < numTickets; i++) {
    const keyPair = KeyPair.fromRandom("ed25519");
    keyPairs.push(keyPair);
    const publicKey = keyPair.publicKey.toString();
    const questions = event.questions || [];
    let answers: { [key: string]: string } = {};

    for (const question of questions) {
      if (question.required && Math.random() > 0.5) {
        answers[question.question] = `My Answer To: ${question.question}`;
      }
    }
    keyData.push({
      public_key: publicKey,
      metadata: JSON.stringify({
        questions: answers,
      }),
      key_owner: Math.random() > 0.5 ? "owner.testnet" : undefined,
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
    allKeyData[dropId] = keyPairs.map((kp) => kp.toString()); // Assuming keyPairs needs to be converted to string
    console.log("Added Keys: ", keyPairs);
    console.log("All Keys: ", allKeyData);
  } catch (e) {
    console.log("(Add Tix) ERROR!!!: ", e);
  }
};

const main = async () => {
  const near = await initNear();
  const signerAccount = await near.account("benjiman.testnet");
  const keypomAccountId = `${Date.now().toString()}-kp-ticketing.testnet`;
  console.log("Deploying Keypom contract to: ", keypomAccountId);
  await deployEventContract({
    signerAccount,
    newAccountId: keypomAccountId,
    amount: "15",
    near,
    wasmPath: "./out/keypom.wasm",
  });
  const marketplaceAccountId = `${Date.now().toString()}-marketplace.testnet`;
  console.log("Creating marketplace: ", marketplaceAccountId);
  await createAccount({
    signerAccount,
    newAccountId: marketplaceAccountId,
    amount: "200",
  });
  const marketplaceAccount = await near.account(marketplaceAccountId);

  const events = generateEvents(50);
  let nonce = 0; // Initialize a nonce variable

  // Process each event sequentially
  for (const event of events) {
    console.log("Deploying Event: ", event.eventInfo.name);

    // Sequentially process each ticket with a delay
    for (const ticket of event.tickets) {
      nonce += 1;
      const dropId = `${Date.now().toString()}-${
        ticket.ticketInfo.name
      }-${nonce}`;
      // Call createTicket and wait for it to complete with a delay afterwards
      await createTicket({
        // Parameters for createTicket
        signerAccount,
        keypomAccountId,
        marketplaceAccount,
        marketplaceAccountId,
        dropId,
        ticket,
        event: event.eventInfo,
        nonce,
      });
      await delay(1000); // Delay to prevent nonce retries exceeded error
    }

    console.log("Deployed Event: ", event.eventInfo.name);
  }

  for (const event of events) {
    console.log(
      `Event ${event.eventInfo.id} ( ${event.eventInfo.name} has ${event.tickets.length} tickets)`,
    );
  }
  // Log completion and provide a link to the contract
  console.log(
    `All events deployed. Check them out at: https://testnet.nearblocks.io/address/${keypomAccountId}`,
  );

  // Write the accumulated key data to a file
  console.log("Writing key pairs to file...", allKeyData);
  const filePath = path.join(__dirname, "keyPairs.json");
  await fs.writeFileSync(filePath, JSON.stringify(allKeyData), "utf-8");
  console.log(`Key pairs written to ${filePath}`);
};

// test();
main().catch(console.error);
