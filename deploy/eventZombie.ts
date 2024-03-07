import {
  DropMetadata,
  FunderEventMetadata,
  FunderMetadata,
  QuestionInfo,
} from "./interfaces";
import {
  addTickets,
  createAccount,
  createContracts,
  decryptPrivateKey,
  decryptWithPrivateKey,
  deriveKeyFromPassword,
  encryptPrivateKey,
  encryptWithPublicKey,
  exportPublicKeyToBase64,
  generateEvents,
  generateKeyPair,
  initNear,
  sendTransaction,
  uint8ArrayToBase64,
} from "./utils";
const { KeyPair, utils } = require("near-api-js");
import * as crypto from "crypto";



const fs = require("fs");
const path = require("path");
let allKeyData: { [key: string]: string[] } = {};

const main = async () => {
  const near = await initNear();
  const createAccounts = true;

  const signerAccount = await near.account("benjiman.testnet");
  const masterKey = "MASTER_KEY";

  let keypomContractId = `1709832679459-kp-ticketing.testnet`;
  let marketplaceContractId = `1709145202470-marketplace.testnet`;
  if (createAccounts) {
    keypomContractId = `${Date.now().toString()}-kp-ticketing.testnet`;
    marketplaceContractId = `${Date.now().toString()}-marketplace.testnet`;
    await createContracts({
      signerAccount,
      near,
      marketplaceContractId,
      keypomContractId,
    });
  }

  const marketAccount = await near.account(marketplaceContractId);

  //  Create Events (and generate keypair if necessary / update user metadata)
  // To store: public key, encrypted private key, iv, salt
  const events = generateEvents(1);
  let nonce = 0;
  let funderMetadata: FunderMetadata = {};

  let allTickets: Array<{
    dropId: string;
    ticket: DropMetadata;
    eventId: string;
    eventQuestions?: QuestionInfo[];
  }> = [];
  for (const event of events) {
    console.log("Deploying Event: ", event.eventMeta.name);
    if ((event.eventMeta.questions || []).length > 0) {
      console.log("Event has questions. Generate keypairs");
      const { publicKey, privateKey } = await generateKeyPair();
      const saltBytes = crypto.getRandomValues(new Uint8Array(16));
      const saltBase64 = uint8ArrayToBase64(saltBytes);
      const symmetricKey = await deriveKeyFromPassword(masterKey, saltBase64);
      const { encryptedPrivateKeyBase64, ivBase64 } = await encryptPrivateKey(
        privateKey,
        symmetricKey,
      );

      event.eventMeta.pubKey = await exportPublicKeyToBase64(publicKey);
      event.eventMeta.encPrivKey = encryptedPrivateKeyBase64;
      event.eventMeta.iv = ivBase64;
      event.eventMeta.salt = saltBase64;
    }

    funderMetadata[event.eventMeta.id] = event.eventMeta;

    let drop_ids: string[] = [];
    let drop_configs: any = [];
    let asset_datas: any = [];
    let ticket_information: any = [];
    let base_price: number = 1;
    
    for (const ticket of event.tickets) {
      nonce += 1;
      const dropId = `${Date.now().toString()}-${ticket.name}-${nonce}`;
      ticket_information.push({
        [`${dropId}`]: {
          max_tickets: Math.floor(Math.random() * 20) + 10,
          price: utils.format.parseNearAmount(base_price.toString())
        }
      })
      base_price += 1;
      allTickets.push({
        dropId,
        ticket,
        eventId: event.eventMeta.id,
        eventQuestions: event.eventMeta.questions,
      });
      const dropConfig = {
        metadata: JSON.stringify(ticket),
        add_key_allowlist: [marketplaceContractId],
        transfer_key_allowlist: [marketplaceContractId],
      };
      const assetData = [
        {
          uses: 2,
          assets: [null],
          config: {
            permissions: "claim",
          },
        },
      ];
      drop_ids.push(dropId);
      asset_datas.push(assetData);
      drop_configs.push(dropConfig);
    }

    await sendTransaction({
      signerAccount,
      receiverId: keypomContractId,
      methodName: "create_drop_batch",
      args: {
        drop_ids,
        drop_configs,
        asset_datas,
        change_user_metadata: JSON.stringify(funderMetadata),
        on_success: {
          receiver_id: marketplaceContractId,
          method_name: "create_event",
          args: {
            event_id: event.eventMeta.id,
            funder_id: signerAccount.accountId,
            ticket_information
          },
          attached_deposit: "5",
        }
      },
      deposit: "15",
      gas: "300000000000000",
    });

    console.log("Deployed Event: ", event.eventMeta.name);
  }

  let allKeyData: { [key: string]: string[] } = {};
  for (const curTicket of allTickets) {
    const { dropId, eventId, ticket, eventQuestions } = curTicket;
    const keyPairs = await addTickets({
      signerAccount,
      funderAccountId: "benjiman.testnet",
      keypomAccountId: keypomContractId,
      marketplaceAccount: marketAccount,
      dropId,
      ticket,
      eventId,
      eventQuestions,
    });
    allKeyData[dropId] = keyPairs;
  }

  for (const event of events) {
    console.log(
      `Event ( ${event.eventMeta.name} has ${event.tickets.length} tickets)`,
    );
  }

  console.log(
    `All events deployed. Check them out at: https://testnet.nearblocks.io/address/${keypomContractId}`,
  );

  return;
};

async function test() {
  const near = await initNear();
  let keypomContractId = `1709834705601-kp-ticketing.testnet`;
  const signerAccount = await near.account("benjiman.testnet");
  const eventId = "29f2f55e-be89-43e8-aa27-da49cbbed43b";

  const funderInfo = await signerAccount.viewFunction(
    keypomContractId,
    "get_funder_info",
    { account_id: signerAccount.accountId },
  );

  const funderMeta: FunderMetadata = JSON.parse(funderInfo.metadata);
  const eventInfo = funderMeta[eventId];
  console.log("Event Info: ", eventInfo);
}

test();
// main().catch(console.error);
