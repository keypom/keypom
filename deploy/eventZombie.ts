import {
  FunderMetadata,
  QuestionInfo,
  TicketInfoMetadata,
  TicketMetadataExtra,
  ZombieDropMetadata,
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

const main = async () => {
  const near = await initNear();
  const createAccounts = true;

  const signerAccount = await near.account("benjiman.testnet");
  const masterKey = "MASTER_KEY";

  let keypomContractId = `1710351544642-kp-ticketing.testnet`;
  let marketplaceContractId = `1710351544642-marketplace.testnet`;
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
  const keypomAccount = await near.account(keypomContractId);

  //  Create Events (and generate keypair if necessary / update user metadata)
  // To store: public key, encrypted private key, iv, salt
  const events = generateEvents(20);
  let nonce = 0;
  const funderInfo = await signerAccount.viewFunction(
    keypomContractId,
    "get_funder_info",
    { account_id: signerAccount.accountId },
  );

  const funderMetadata: FunderMetadata =
    funderInfo == undefined ? {} : JSON.parse(funderInfo.metadata); // initialize this to whatever the funder metadata currently is
  let allTickets: Array<{
    dropId: string;
    ticket: ZombieDropMetadata;
    eventId: string;
    eventQuestions?: QuestionInfo[];
  }> = [];
  for (const event of events) {
    try {
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
      let ticket_information: { [key: string]: any } = {};
      let base_price: number = 1;

      let totalExcessBytes = 0;
      for (const ticket of event.tickets) {
        nonce += 1;
        const dropId = `${Date.now().toString()}-${ticket.name}-${nonce}`;

        ticket_information[`${dropId}`] = {
          max_tickets: ticket.maxSupply,
          price: ticket.price,
          sale_start: Date.now(),
          sale_end: Date.now() + 1000 * 60 * 60 * 24 * 2,
        };
        base_price += 1;

        allTickets.push({
          dropId,
          ticket,
          eventId: event.eventMeta.id,
          eventQuestions: event.eventMeta.questions,
        });

        const ticketMetadataExtra: TicketMetadataExtra = {
          eventId: event.eventMeta.id,
          dateCreated: Date.now().toString(),
          salesValidThrough: ticket.salesValidThrough,
          passValidThrough: ticket.passValidThrough,
          price: ticket.price,
          maxSupply: ticket.maxSupply,
        };
        const nftMetadata: TicketInfoMetadata = {
          title: ticket.name,
          description: ticket.description,
          media: ticket.artwork,
          extra: JSON.stringify(ticketMetadataExtra),
        };
        const dropConfig = {
          nft_keys_config: {
            token_metadata: nftMetadata,
          },
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

      console.log(
        `Creating event with ticket information: ${JSON.stringify(
          ticket_information,
        )}`,
      );

      const funderMetadataString = JSON.stringify(funderMetadata);
      console.log(
        "Funder Metadata: ",
        funderMetadataString,
        " With length: ",
        funderMetadataString.length,
      );

      console.log("Total Excess Bytes: ", totalExcessBytes);

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
            args: JSON.stringify({
              event_id: event.eventMeta.id,
              funder_id: signerAccount.accountId,
              ticket_information,
            }),
            attached_deposit: utils.format.parseNearAmount("1"),
          },
        },
        deposit: "15",
        gas: "300000000000000",
      });

      console.log("Deployed Event: ", event.eventMeta.name);
    } catch (e) {
      console.error("Error deploying event: ", e);
    }
  }

  let allKeyData: { [key: string]: string[] } = {};
  for (const curTicket of allTickets) {
    try {
      const { dropId, eventId, ticket, eventQuestions } = curTicket;
      const keyPairs = await addTickets({
        signerAccount,
        funderAccountId: signerAccount.accountId,
        keypomAccountId: keypomContractId,
        marketplaceAccount: marketAccount,
        dropId,
        ticket,
        eventId,
        eventQuestions,
      });

      allKeyData[dropId] = keyPairs;
    } catch (e) {
      console.error("Error adding tickets: ", e);
    }
  }

  // Loop through key data and print
  for (const dropId in allKeyData) {
    console.log(`Drop ID: ${dropId}`);
    for (const secretKey of allKeyData[dropId]) {
      console.log(
        `http://localhost:3000/tickets/ticket/${dropId}#secretKey=${secretKey}`,
      );
    }
  }

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

//test();
main().catch(console.error);
