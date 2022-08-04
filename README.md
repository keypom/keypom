<p align="center">
  <img src="assets/claimed-linkdrop.png" alt="Logo" style="width: 35%; height: 35%">
  <br />
</p>

<div align="center">
  <h1>
  Keypom
  </h1>
  Limitless possibilities in the palm of your hand.
</div>

<div align="center">
<br />

[![made by BenKurrek](https://img.shields.io/badge/made%20by-BenKurrek-ff1414.svg?style=flat-square)](https://github.com/BenKurrek)
[![made by mattlockyer](https://img.shields.io/badge/made%20by-MattLockyer-ff1414.svg?style=flat-square)](https://github.com/mattlockyer)


</div>

<details open="open">
<summary>Table of Contents</summary>

- [About](#about)
- [How it Works](#how-it-works)
  - [NFTs](#nft-linkdrops)
  - [Fungible Tokens](#fungible-token-linkdrops)
  - [Function Calls](#function-calls)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Quickstart](#quickstart)  
- [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)

</details>

---

# About

<table>
<tr>
<td>

Keypom sheds light on the endless power that NEAR's access keys introduce. The contract was created as a result of 3 common problems that arose in the ecosystem.

1. People want a cheap, customizable, and unique onboarding experience for users.
2. Companies don't want to expose full access keys in their backend servers.
3. dApps want a smooth UX for interactions that require deposits.

The contract was initially created as a way to handle the 1 $NEAR minimum deposit required for creating linkdrops using the [regular linkdrop contract](https://github.com/near/near-linkdrop/blob/f24f2608e1558db773f2408a28849d330abb3881/src/lib.rs#L18). 

If users wanted to create linkdrops, they needed to attach a **minimum** of 1 $NEAR. This made it costly and unscalable for projects that wanted to mass onboard onto NEAR. 

Keypom, on the other hand, has been highly optimized and the design can be broken down into two categories: keys, and drops. At the end of the day, Keypom is a utility used to generate keys that belong to a drop. The drop outlines a suite of different permissions and outcomes that each key will derive.

At a high level, Keypom allows for 4 different types of drops, each equipped with their own set of customizable features.

1. Simple drops
2. Non Fungible Token drops
3. Fungible Token drops
4. Function Call drops.

# Customizable Features

Keypom provides a highly customizable set of features for keys to inherit from when drops are created. These features come in the form of an optional `DropConfig` and optional `DropMetadata` and are shared across all drops regardless of their types. There are other configurations available specific to certain drop types as well.

## Drop Config

The drop config outlines global configurations that **all** the keys in the drop will inherit from. These configurations are outlined below.
- **`uses_per_key`**: How many times can a key be used before it's deleted.
- **`start_timestamp`**: At what block timestamp can the key **first** be used.
- **`throttle_timestamp`**: How much time must pass in between key uses.
- **`on_claim_refund_deposit`**: If a key was used to call `claim` instead of to create an account, should the key deposit be sent back to the drop owner?
- **`claim_permission`**: What permissions should the key have. This can be either to only call `claim`, `create_account_and_claim`, or both.
- **`drop_root`**: When `create_account_and_claim` is called, accounts normally inherit from the global root (`near` or `testnet`) in order to accounts to be top-level. By overloading this and passing in a `drop_root`, your application can force all created accounts for your drop to be sub-accounts of the `drop_root`. Keep in mind that the `drop_root` specified must have a valid contract deployed to it such that it can create sub-accounts.

## Drop Metadata

In addition to the drop config, the drop metadata is a way to pass additional information about the drop in the form of an arbitrary string. It's up to the drop owner to decide how this information should be used. A common approach is to pass in stringified JSON outlining a title, description, and media for the drop such that it can be rendered nicely on frontends.

## NFT and FT Configs

When creating either an NFT or FT drop, the creator has the ability to specify 2 different fields:
- **`contract_id`**: What token contract will the drop use?
- **`sender_id`**: Who will be sending the tokens to the contract?

FT Specific:
- **`balance_per_use`**: How many tokens will be sent per key use?

NFT Specific:
- **`longest_token_id`**: What is the longest token ID that will be used in the drop? (this is for storage optimizations and is explained in the [Cost](#cost) section)

## Function Call Configurations

Keypom allows for suite of features when creating function call drops. This allows for almost endless possibilities for creators. At the top level, each drop will have an optional `FCConfig` outlining features that all keys will inherit from.
- **`account_id_field`**: When a key is used, what field should the claimed account be added to in the function call args? For example, if `benji.near` used the key and the account ID field was set to `receiver_id`, the args would have `{ "receiver_id": "benji.near" }` added.
- **`drop_id_field`**: When a key is used, what field should the drop ID be added to in the function call args? For example, if a key belonging to drop ID `0` was used and the drop ID field was set to `id`, the args would have `{ "id": "0" }` added.
- **`key_id_field`**: When a key is used, what field should the key ID be added to in the function call args? For example, if a key with ID `13` was used and the key ID field was set to `key`, the args would have `{ "key": "13" }` added.
- **`attached_gas`**: How much gas should be attached to the function call? If this is specified, the key can **only** be used to call `claim`.

In addition to the FCConfig, the creator can specify what's known as `MethodData`. This data outlines the specifics of the functions executed when a key is used.
- **`receiver_id`**: What contract should the function call be sent to?
- **`method_name`**: What method name should be called?
- **`args`**: What arguments should be passed to the function call?
- **`attached_deposit`**: How much deposit should be attached to the function call?

This method data is outlined in the form of a set of optional `MethodData` vectors. Everytime a key is used, if the Method Data is null, it will be skipped and the uses are decremented. If the Method Data is not null, the contract will execute all functions in the vector. If only 1 vector of Method Data is defined, that will be used for all uses.

Let's look at an example of how powerful this can be. Let's say you're doing an NFT ticketing event and want to have a proof of attendance where users will have an NFT lazy minted to them if they actually show up to the event. 

You could have a key with 2 claims where the first method data is null and the second is a vector of size 1 that will lazy mint an NFT. You could setup an app that claims the null case when the person visits the link you gave them. The bouncer could then give them a password that would allow them to claim the second use and get the NFT. They can only do this if they show up to the event and get the password from the bouncer as the link you gave them is encrypted. As the creator, you would know how many people didn't use your original link, used it but didn't show up, and showed up all by checking the uses of the key.

</p>

# Cost

There are several costs that must be taken into account when using Keypom. These costs are broken down into two categories: per key and per drop. On top of these costs, Keypom takes **1 $NEAR** per drop and **0.005 $NEAR** per key. This model promotes drops with a lot of keys rather than many different drops with fewer keys. These numbers **can** be changed on a per-account basis so reach out to Ben or Matt if this is of interest to your application. 

> **NOTE:** Creating an empty drop and then adding 100 keys in separate calls will incur the same cost as creating a drop with 100 keys in the same call.

## Per Drop

When creating an empty drop, there are only two costs to keep in mind regardless of the drop type:
- Keypom's drop fee (**1 $NEAR**)
- Storage cost (**~0.008 $NEAR** for simple drops)

## Per Key
Whenever keys are added to a drop (either when the drop is first created or at a later date), the costs are outlined below.

### Key Costs for Simple Drop

- Keypom's key fee (**0.005 $NEAR**).
- $NEAR sent when the key is used (can be 0).
- Access key allowance (**~0.0187 $NEAR per use**).
- Storage for creating access key (**0.001 $NEAR**).
- Storage cost (**~0.006 $NEAR** for simple drops)

### Additional Costs for NFT Drops

Since keys aren't registered for use until **after** the contract has received the NFT, we don't know how much storage the token IDs will use on the contract. To combat this, the drop creators must pass in the **longest token ID** and the contract will charge that storage cost for all key uses.

### Additional Costs for FT Drops

Since accounts claiming FTs may or may not be registered on the Fungible Token contract, Keypom will automatically try to register **all** accounts. This means that the drop creators must front the cost of registering users depending on the `storage_balance_bounds` returned from the FT contract. This applies to every use for every key.

### Additional Costs for FC Drops

Drop creators have a ton of customization available to them when creation Function Call drops. A cost that they might incur is the attached deposit being sent alongside the function call. Keypom will charge creators for all the attached deposits they specify.

> **NOTE:** The storage costs are dynamically calculated and will vary depending on the information you store on-chain.

## Deleting Keys and Drops

Creators have the ability to delete drops and keys at any time. In this case, **all** the initial costs they incurred for the remaining keys will be refunded to them except for Keypom's fees.

## Automatic Refunds When Keys are Used

One way that Keypom optimizes the fee structure is by performing automatic refunds for some of the initial costs that creators pay for when keys are used. All the storage that is freed along with any unused allowance is automatically sent back to the creator whenever a key is used. This model drastically reduces the overall costs of creating drops and creates incentives for the keys to be used. 

## Account Balances for Smooth UX

In order to make the UX of using Keypom seamless, the contract introduces a debit account model. All costs and refunds go through your account's balance which is stored on the contract. This balance can be topped up or withdrawn at any moment using the `add_to_balance()`  and `withdraw_from_balance()` functions.

</td>
</tr>
</table>

## Built With

- [near-sdk-rs](https://github.com/near/near-sdk-rs)
- [near-api-js](https://github.com/near/near-api-js)

# How Linkdrops Works

For some background as to how linkdrops works on NEAR: 

*The funder that has an account and some $NEAR:* 
- creates a keypair locally `(pubKey1, privKey1)`. The blockchain doesn't know of this key's existence yet since it's all local for now.
- calls `send` on the contract and passes in the `pubKey1` as an argument as well as the desired `balance` for the linkdrop.
    - The contract will map the `pubKey1` to the desired `balance` for the linkdrop.
    - The contract will then add the `pubKey1` as a **function call access key** with the ability to call `claim` and `create_account_and_claim`. This means that anyone with the `privKey1` that was created locally, can claim this linkdrop. 
- Funder will then create a link to send to someone that contains this `privKey1`. The link follows the following format: 
```
    wallet.testnet.near.org/linkdrop/{fundingContractAccountId}/{linkdropKeyPairSecretKey}?redirectUrl={redirectUrl}
```
* `fundingContractAccountId`: The contract accountId that was used to send the funds.
* `linkdropKeyPairSecretKey`: The corresponding secret key to the public key sent to the contract.
* `redirectUrl`: The url that wallet will redirect to after funds are successfully claimed to an existing account. The URL is sent the accountId used to claim the funds as a query param.

*The receiver of the link that is claiming the linkdrop:* 
- Receives the link which includes `privKey1` and sends them to the NEAR wallet.
- Wallet creates a new keypair `(pubKey2, privKey2)` locally. The blockchain doesn't know of this key's existence yet since it's all local for now.
- Receiver will then choose an account ID such as `new_account.near`. 
- Wallet will then use the `privKey1` which has access to call `claim` and `create_account_and_claim` in order to call `create_account_and_claim` on the contract.
    - It will pass in `pubKey2` which will be used to create a full access key for the new account.
- The contract will create the new account and transfer the funds to it alongside any NFT or fungible tokens pre-loaded.

</p>

## NFT Linkdrops

With Keypom contract, users can pre-load each key with a set of NFTs depending on how many uses per key. Each use will pop the last token ID off and send it to the claimed account.

In order to pre-load the NFT and register a key use, you must:
- create a linkdrop either through `send` or `send_multiple` and specify the NFTData for the NFT that will be pre-loaded onto the linkdrop. The NFT Data struct can be seen below.

```rust
pub struct NFTData {
    pub nft_sender: String,
    pub nft_contract: String,
    pub nft_token_id: String,
}
```

An example of creating an NFT linkdrop can be seen:

```bash
near call linkdrop-proxy.testnet send '{"public_key": "ed25519:2EVN4CVLu5oH18YFoxGyeVkg1c7MaDb9aDrhkaWPqjd7", "balance": "2840000000000000000000", "nft_data": {"nft_sender": "benjiman.testnet", "nft_contract": "nft.examples.testnet", "nft_token_id": "token1"}}' --accountId "benjiman.testnet" --amount 1
```

- Once the regular linkdrop has been created with the specified NFT Data, execute the `nft_transfer_call` funtion on the NFT contract and you *must* pass in `pubKey1` (the public key of the keypair created locally and passed into the `send` function) into the `msg` parameter. If the linkdrop is claimed before activation, it will act as a regular linkdrop with no NFT. 

```bash
near call nft.examples.testnet nft_transfer_call '{"token_id": "token1", "receiver_id": "linkdrop-proxy.testnet", "msg": "ed25519:4iwBf6eAXZ4bcN6TWPikSqu3UJ2HUwF8wNNkGZrgDYqE"}' --accountId "benjiman.testnet" --depositYocto 1
```

> **NOTE:** you must send the NFT after the linkdrop has been created. You cannot send an NFT with a public key that isn't on the contract yet. The NFT must match exactly what was specified in the NFT data when creating the linkdrop.

<p align="center">
  <img src="flowcharts/adding-nfts-and-fts-to-linkdrops.png" style="width: 65%; height: 65%" alt="Logo">
</p>

Once the NFT is sent to the contract, it will be registered and you can view the current information about any key using the `get_key_information` function. Upon claiming, the NFT will be transferred from the contract to the newly created account (or existing account) along with the balance of the linkdrop. If any part of the linkdrop claiming process is unsuccessful, **both** the NFT and the $NEAR will be refunded to the funder and token sender respectively.

> **NOTE:** If the NFT fails to transfer from the contract back to the token sender due to a refund for any reason, the NFT will remain on the proxy contract.

If the linkdrop is successfully claimed, the funder will be refunded for everything **except** the burnt GAS and linkdrop balance. This results in the actual linkdrop cost being extremely low (burnt GAS + initial balance).

<p align="center">
  <img src="flowcharts/claiming-nft-linkdrops-with-new-accounts.png" style="width: 65%; height: 65%" alt="Logo">
</p>

## Fungible Token Linkdrops

With the proxy contract, users can pre-load a linkdrop with **only one** type of fungible token due to GAS constraints. The number of fungible tokens, however, is not limited. You could load 1 TEAM token, or a million TEAM tokens. You cannot, however, load 10 TEAM tokens and 50 MIKE tokens at the same time.

Due to the nature of how fungible token contracts handle storage, the user is responsible for attaching enough $NEAR to cover the registration fee. As mentioned in the [About](#about) section, this amount is dynamically calculated before the linkdrop is created in the `send` or `send_multiple` functions. The process for creating fungible token linkdrops is very similar to the NFT linkdrops:

- create a linkdrop either through `send` or `send_multiple` and specify the FTData for the Fungible Tokens that will be pre-loaded onto the linkdrop. The FT Data struct can be seen below.

```rust
pub struct FTData {
    pub ft_contract: String,
    pub ft_sender: String,
    pub ft_balance: U128, // String
    pub ft_storage: Option<U128>, // String
}
```

An example of creating an FT linkdrop can be seen:

```bash
near call linkdrop-proxy.testnet send '{"public_key": "ed25519:2EVN4CVLu5oH18YFoxGyeVkg1c7MaDb9aDrhkaWPqjd7", "balance": "2840000000000000000000", "ft_data": {"ft_sender": "benjiman.testnet", "ft_contract": "ft.benjiman.testnet", "ft_balance": "25"}}' --accountId "benjiman.testnet" --amount 1
```

Once the regular linkdrop is created with the fungible token data, you can the send the fungible tokens to activate the linkdrop. If the linkdrop is claimed before activation, it will act as a regular linkdrop with no FTs.

<p align="center">
  <img src="flowcharts/adding-nfts-and-fts-to-linkdrops.png" style="width: 65%; height: 65%" alt="Logo">
</p>

Once the regular linkdrop is created with the fungible token data, you can the send the fungible tokens to activate the linkdrop. If the linkdrop is claimed before activation, it will act as a regular linkdrop with no FTs.

- execute the `ft_transfer_call` function on the FT contract and you *must* pass in `pubKey1` (the public key of the keypair created locally and passed into the `send` function) into the `msg` parameter. An example of this can be: 

```bash
near call FT_CONTRACT.testnet ft_transfer_call '{"receiver_id": "linkdrop-proxy.testnet", "amount": "25", "msg": "ed25519:4iwBf6eAXZ4bcN6TWPikSqu3UJ2HUwF8wNNkGZrgDYqE"}' --accountId "benjiman.testnet" --depositYocto 1
```

> **NOTE:** you must send the FT after the linkdrop has been created. You cannot send FTs with a public key that isn't on the contract yet. You are also responsible for registering the proxy contract for the given fungible token contract if it isn't registered already.

Once the fungible tokens are sent to the contract, they will be registered and you can view the current information about any key using the `get_key_information` function. Upon claiming, the proxy contract will register the newly created account (or existing account) on the fungible token contract using the storage you depositted in the `send` function. After this is complete, the fungible tokens will be transferred from the contract to the claimed account along with the balance of the linkdrop. If any part of the linkdrop claiming process is unsuccessful, **both** the fungible tokens and the $NEAR will be refunded to the funder and token sender respectively.

> **NOTE:** If the FT fails to transfer from the contract back to the token sender due to a refund for any reason, the fungible tokens will remain on the proxy contract.

If the linkdrop is successfully claimed, the funder will be refunded for everything **except** the burnt GAS, linkdrop balance, and fungible token storage.

<p align="center">
  <img src="flowcharts/claiming-ft-linkdrops-with-new-accounts.png"  style="width: 65%; height: 65%" alt="Logo">
  <br />
</p>

## Function Calls

With the proxy contract, users can specify a function that will be called when the linkdrop is claimed. This function call is highly customizable including:
- Any method on any contract
- Any deposit to attach to the call
- Whether or not the refund that normally goes to the funder should be sent along with the deposit
- Specifying a specific field for the claiming account to be called with.

Let's look at an example to see the power of the proxy contract. If a user wants to be able to lazy mint an NFT to the newly created account (that is unknown at the time of creating the linkdrop) but the mint function takes a parameter `receiver_id` and a deposit of 1 $NEAR, you could specify these parameters. The struct that must be passed in when creating a function call linkdrop is below.

```rust
pub struct FCData {
    // Contract that will be called
    pub receiver: String,
    // Method to call on receiver contract
    pub method: String,
    // Arguments to pass in (stringified JSON)
    pub args: String,
    // Amount of yoctoNEAR to attach along with the call
    pub deposit: U128,
    // Should the refund that normally goes to the funder be attached alongside the deposit?
    pub refund_to_deposit: Option<bool>,
    // Specifies what field the claiming account should go in when calling the function
    pub claimed_account_field: Option<String>,
}
```

If there was a different NFT contract where the parameter was `nft_contract_id` instead, that is also possible. You can specify the exact field that the claiming account ID should be passed into. An example flow of creating a function call linkdrop is below.

- create a linkdrop either through `send` or `send_multiple` and specify the FC (function call) data for the function that will be called upon claim

```bash
near call linkdrop-proxy.testnet send '{"public_key": "ed25519:2EVN4CVLu5oH18YFoxGyeVkg1c7MaDb9aDrhkaWPqjd7", "balance": "2840000000000000000000", "fc_data": {"receiver": "nft.examples.testnet", "method": "nft_mint", "args": "{\"token_id\":\"ed25519:Db3ALuBMU2ruMNroZfwFC5ZGMXK3bRX12UjRAbH19LZL\",\"token_metadata\":{\"title\":\"My Linkdrop Called This Function!\",\"description\":\"Linkdrop NFT that was lazy minted when the linkdrop was claimed\",\"media\":\"https://bafybeicek3skoaae4p5chsutjzytls5dmnj5fbz6iqsd2uej334sy46oge.ipfs.nftstorage.link/\",\"media_hash\":null,\"copies\":10000,\"issued_at\":null,\"expires_at\":null,\"starts_at\":null,\"updated_at\":null,\"extra\":null,\"reference\":null,\"reference_hash\":null}}", "deposit": "1000000000000000000000000", "refund_to_deposit": true, "claimed_account_field": "receiver_id" }}' --accountId "benjiman.testnet" --amount 1
```

This will create a linkdrop for `0.00284 $NEAR` and specify that once the linkdrop is claimed, the method `nft_mint` should be called on the contract `nft.examples.testnet` with a set of arguments that are stringified JSON. In addition, an **extra field called receiver_id** should be **added to the args** and the claiming account ID will be set for that field in the arguments.

> **NOTE:** you must attach enough $NEAR to cover the attached deposit. If the linkdrop claim fails, your $NEAR will be refunded and the function call will NOT execute.

<p align="center">
  <img src="flowcharts/claiming-function-call-linkdrops-with-new-accounts.png" style="width: 65%; height: 65%" alt="Logo">
</p>

# Getting Started

## Prerequisites

In order to successfully use this contract, you should have the following installed on your machine: 


- [NEAR account](https://docs.near.org/docs/develop/basics/create-account)
- [rust toolchain](https://docs.near.org/docs/develop/contracts/rust/intro#installing-the-rust-toolchain)
- [NEAR CLI](https://docs.near.org/docs/tools/near-cli#setup)

If you want to run the deploy scripts, you'll need:
- [Node JS](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm)

## Quickstart

The project comes with several useful scripts in order to test and view functionalities for creating linkdrops. Each script can be set to either batch create linkdrops or create them one by one:

- [simple.js](deploy/simple.js) creating linkdrops preloaded with just $NEAR
- [nft.js](deploy/nft.js) creating linkdrops preloaded with $NEAR and an NFT
- [ft.js](deploy/ft.js) creating linkdrops preloaded with $NEAR and fungible tokens.
- [function-call.js](deploy/funtion-call.js) creating linkdrops preloaded with $NEAR and fungible tokens.

In addition, there is a test script that will create a function call recursive linkdrop that keeps calling the contract to create a new linkdrop once the old one is claimed. To test it out, visit the [recursive-fc.js](deploy/recursive-fc.js) script.

The first step is to compile the contract to WebAssembly by running:

```
yarn build-contract
```
This will create the directory `out/main.wasm` where you can then deploy the contract using:

```
near deploy --wasmFile out/main.wasm --accountId YOUR_CONTRACT_ID.testnet
```

> **NOTE:** you must replace `YOUR_CONTRACT_ID.testnet` with the actual NEAR account ID you'll be using.


Once deployed, you need to initialize the contract with the external linkdrop contract you want to interact with. In most cases, this will be `near` or `testnet` since you'll want to create sub-accounts of `.testnet` (i.e `benjiman.testnet`).

```
near call YOUR_CONTRACT_ID.testnet new '{"linkdrop_contract": "testnet", "owner_id": "YOUR_CONTRACT_ID.testnet"}' --accountId YOUR_CONTRACT_ID.testnet
```

You're now ready to create custom linkdrops! You can either interact with the contract directly using the CLI or use one of the pre-deployed scripts.

## Using the CLI
After the contract is deployed, you have a couple options for creating linkdrops: 

- Creating single linkdrops.
- Creating multiple linkdrops at a time.

This will cover creating single linkdrops, however, the only differences between `send` and `send_multiple` are outlined in the [how it works](#how-it-works) flowchart section. 

- Start by creating a keypair locally (you can use near-api-js to do this as seen in the deploy scripts).
- Call the `send` function and pass in the `public_key`, `balance`. If creating a FT, NFT, or FC linkdrop, you must specify the struct as well. This is outlined in the respective sections.  

```bash
near call YOUR_CONTRACT_ID.testnet send '{"public_key": "ed25519:4iwBf6eAXZ4bcN6TWPikSqu3UJ2HUwF8wNNkGZrgDYqE", "balance": "10000000000000000000000"}' --deposit 1 --accountId "benjiman.testnet"
```

Once the function is successful, you can create the link and click it to claim the linkdrop:
```
    wallet.testnet.near.org/linkdrop/{YOUR_CONTRACT_ID.testnet}/{privKey1}
```

## Using the pre-deployed scripts

If you'd like to use some of the deploy scripts found in the `deploy` folder, those can help automate the process. 
<br />

### Simple Linkdrops with No NFTs or FTs

If you'd like to create a simple linkdrop with no pre-loaded NFTs or FTs, first specify the following environment variables:

```bash
export LINKDROP_PROXY_CONTRACT_ID="INSERT_HERE"
export FUNDING_ACCOUNT_ID="INSERT_HERE"
export LINKDROP_NEAR_AMOUNT="INSERT_HERE"
export SEND_MULTIPLE="false"
```

This will set the proxy contract that you wish to create linkdrops on, the account ID of the funding address (person creating the linkdrops and sending the funds), the actual $NEAR amount that the linkdrop will contain and whether or not to batch create linkdrops. By default, if the batch option is true, it will create 5 linkdrops. 

It is recommended to simply run a `dev-deploy` and use the dev contract ID to test these scripts. Once this is finished, run the following script:

```
node deploy/simple.js
```

Once the script has finished executing, a link to the wallet should appear in your console similar to: 

```bash
https://wallet.testnet.near.org/linkdrop/dev-1652794689263-24159113353222/4YULUt1hqv4s96Z8K83VoPnWqXK9vjfYb5QsBrv793aZ2jucBiLP35YWJq9rPGziRpDM35HEUftUtpP1WLzFocqJ
```

Once you've clicked the link, you can either fund an existing account with the linkdrop balance, or you can create a new account and fund it that way.
<br />

### Linkdrops with NFTs

If you'd like to create a linkdrop with a pre-loaded NFT, first specify the following environment variables:

```bash
export LINKDROP_PROXY_CONTRACT_ID="INSERT_HERE"
export FUNDING_ACCOUNT_ID="INSERT_HERE"
export LINKDROP_NEAR_AMOUNT="INSERT_HERE"
export SEND_MULTIPLE="false"
```

If you ran the script now, it would mint a predefined NFT on the contract `nft.examples.testnet`. If you wish to change the NFT contract or the metadata for the token, simply open the `deploy/nft.js` script and change the following lines:

```js
/*
	Hard coding NFT contract and metadata. Change this if you want.
*/
let NFT_CONTRACT_ID = "nft.examples.testnet";
const METADATA = {
	"title": "Linkdropped Go Team NFT",
	"description": "Testing Linkdrop NFT Go Team Token",
	"media": "https://bafybeiftczwrtyr3k7a2k4vutd3amkwsmaqyhrdzlhvpt33dyjivufqusq.ipfs.dweb.link/goteam-gif.gif",
	"media_hash": null,
	"copies": 10000,
	"issued_at": null,
	"expires_at": null,
	"starts_at": null,
	"updated_at": null,
	"extra": null,
	"reference": null,
	"reference_hash": null
};
```

Once you've either changed the NFT info or you're happy with minting a Go Team NFT on the example NFT contract, run the NFT script:

```
node deploy/nft.js
```

Once the script has finished executing, a link to the wallet should appear in your console similar to: 

```bash
https://wallet.testnet.near.org/linkdrop/dev-1652794689263-24159113353222/4YULUt1hqv4s96Z8K83VoPnWqXK9vjfYb5QsBrv793aZ2jucBiLP35YWJq9rPGziRpDM35HEUftUtpP1WLzFocqJ
```

Once you've clicked the link, you can either fund an existing account with the linkdrop balance, or you can create a new account and fund it that way. When this is finished, navigate to your collectibles tab and you should see an NFT similar to:

<p align="center">
  <img src="assets/claimed-nft.png"  style="width: 65%; height: 65%" alt="Logo">
  <br />
</p>

## Linkdrops with FTs

If you'd like to create a linkdrop with some pre-loaded FTs, you'll need to first specify the following environment variables:

```bash
export LINKDROP_PROXY_CONTRACT_ID="INSERT_HERE"
export FUNDING_ACCOUNT_ID="INSERT_HERE"
export LINKDROP_NEAR_AMOUNT="INSERT_HERE"
export SEND_MULTIPLE="false"
```

In addition, you need to specify the FT contract ID you'd like to pre-load the linkdrop with.

```bash
export FT_CONTRACT_ID="INSERT_HERE"
```
> **NOTE:** the FT script will pay for the proxy contract's storage but the funding account ID must be in possession of at least 25 FTs or else the script will panic.

Once this is finished, run the FT script.

```
node deploy/ft.js
```

Once the script has finished executing, a link to the wallet should appear in your console similar to: 

```bash
https://wallet.testnet.near.org/linkdrop/dev-1652794689263-24159113353222/4YULUt1hqv4s96Z8K83VoPnWqXK9vjfYb5QsBrv793aZ2jucBiLP35YWJq9rPGziRpDM35HEUftUtpP1WLzFocqJ
```

Once you've clicked the link, you can either fund an existing account with the linkdrop balance, or you can create a new account and fund it that way. When this is finished, you should see your fungible tokens:

<p align="center">
  <img src="assets/claimed-ft.png"  style="width: 65%; height: 65%" alt="Logo">
</p>

### Linkdrops with Function Calls

If you'd like to create a linkdrop whereby a function will be called upon claiming, first specify the following environment variables.

```bash
export LINKDROP_PROXY_CONTRACT_ID="INSERT_HERE"
export FUNDING_ACCOUNT_ID="INSERT_HERE"
export LINKDROP_NEAR_AMOUNT="INSERT_HERE"
export SEND_MULTIPLE="false"
```

This script will lazy mint an NFT once the linkdrop is claimed. Feel free to edit the logic in the script if you'd like to call a different function.

```
node deploy/function-call.js
```

Once the script has finished executing, a link to the wallet should appear in your console similar to: 

```bash
https://wallet.testnet.near.org/linkdrop/dev-1652794689263-24159113353222/4YULUt1hqv4s96Z8K83VoPnWqXK9vjfYb5QsBrv793aZ2jucBiLP35YWJq9rPGziRpDM35HEUftUtpP1WLzFocqJ
```

Once you've clicked the link, you can either fund an existing account with the linkdrop balance, or you can create a new account and fund it that way. When this is finished, navigate to your collectibles tab and you should see an NFT similar to:

<p align="center">
  <img src="assets/claimed-fc-nft.png"  style="width: 45%; height: 45%" alt="Logo">
  <br />
</p>

# Contributing

First off, thanks for taking the time to contribute! Contributions are what makes the open-source community such an amazing place to learn, inspire, and create. Any contributions you make will benefit everybody else and are **greatly appreciated**.

Please try to create bug reports that are:

- _Reproducible._ Include steps to reproduce the problem.
- _Specific._ Include as much detail as possible: which version, what environment, etc.
- _Unique._ Do not duplicate existing opened issues.
- _Scoped to a Single Bug._ One bug per report.

Please adhere to this project's [code of conduct](docs/CODE_OF_CONDUCT.md).

You can use [markdownlint-cli](https://github.com/igorshubovych/markdownlint-cli) to check for common markdown style inconsistency.

# License

This project is licensed under the **GPL License**.

# Acknowledgements

Thanks for these awesome resources that were used during the development of the **Linkdrop Proxy Contract**:

- <https://github.com/dec0dOS/amazing-github-template>
- <https://github.com/near/near-linkdrop>
- <https://github.com/near/near-wallet/blob/master/packages/frontend/docs/Linkdrop.md>
