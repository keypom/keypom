<h1 align="center">
    <img src="assets/claimed-linkdrop.png" alt="Logo" width="314" height="322">
</h1>

<div align="center">
  NEAR Linkdrop Proxy - the hub for creating linkdrops with $NEAR, NFTs, and FTs
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
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Quickstart](#quickstart)  
  - [Usage](#usage)
- [Flowcharts](#flowcharts)

- [Contributing](#contributing)
- [Acknowledgements](#acknowledgements)

</details>

---

## About

<table>
<tr>
<td>

The NEAR linkdrop proxy contract was initially created as a way to handle the hardcoded minimum 1 $NEAR fee for creating linkdrops using the [regular linkdrop contract](https://github.com/near/near-linkdrop/blob/f24f2608e1558db773f2408a28849d330abb3881/src/lib.rs#L18). If users wanted to create linkdrops, they needed to attach a **minimum** of 1 $NEAR. This made it costly and unscalable for projects that wanted to mass create linkdrops for an easy onboarding experience to NEAR.

The proxy contract requires a upfront fee of at minimum ~0.02784 $NEAR, which is **97.216% cheaper** than the alternate solution. Some of this upfront fee will be refunded to the funder once the account is created, thus making it even cheaper.

The ~0.02784 $NEAR comes from: 
- 0.02 $NEAR for the function call access key allowance.
- 0.00284 $NEAR for the base cost of creating an account ID (due to storage). This number is based on the largest possible account ID (64 characters).
- 0.005 $NEAR for storing the account details + key on the proxy contract
- *Optional* storage for registering the new account ID on a fungible token contract. This amount is dependant on the FT contract and is dynamically calculated before a linkdrop is created (in the send function).

> **NOTE:** any excess $NEAR attached to the call that isn't covered by the (desired balance + access key allowance + storage + possibly fungible token storage) will be automatically refunded to the funder

Key features of the **Linkdrop Proxy Contract**:

- **Batch creation** of linkdrops within the contract.
- **Customizable balance** that the linkdrop will contain.
- Ability to pre-load the linkdrop with an **NFT** from **any** NEP-171 compatible smart contract.
- Ability to pre-load the linkdrop with **fungible tokens** from **any** NEP-141 compatible smart contract.
- Extremely **low required deposits** when compared with traditional approaches

</td>
</tr>
</table>

### Built With

- [near-sdk-rs](https://github.com/near/near-sdk-rs)
- [near-api-js](https://github.com/near/near-api-js)

## How it Works

Once the contract is deployed, you can either batch create linkdrops, or you can create them one-by-one. With each basic linkdrop, you have the option to either pre-load them with an NFT, or a fungible token.

For some background as to how the linkdrop proxy contract works on NEAR: 

*The funder that has an account and some $NEAR:* 
- creates a keypair locally `(pubKey1, privKey1)`. The blockchain doesn't know of this key's existence yet since it's all local for now.
- calls `send` on the proxy contract and passes in the `pubKey1` as an argument as well as the desired `balance` for the linkdrop.
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
- Wallet will then use the `privKey1` which has access to call `claim` and `create_account_and_claim` in order to call `create_account_and_claim` on the proxy contract.
    - It will pass in `pubKey2` which will be used to create a full access key for the new account.
- The proxy contract will create the new account and transfer the funds to it alongside any NFT or fungible tokens pre-loaded.

To view information account data information for a given key, you can call the following view function: 

```bash
near view YOUR_LINKDROP_PROXY_CONTRACT get_key_information '{"key": "ed25519:7jszQk7sfbdQy8NHM1EfJi9r3ncyvKa4ZoKU7uk9PbqR"}'
```

<details>
<summary>Example response: </summary>
<p>

```bash
{
  funder_id: 'benjiman.testnet',
  balance: '10000000000000000000000',
  token_sender: 'benjiman.testnet',
  token_contract: 'example-nft.testnet',
  nft_id: '1652719786895',
  ft_balance: null,
  ft_storage: null
}
```

</p>
</details>


### NFT Linkdrops

With the proxy contract, users can pre-load a linkdrop with **only one** NFT due to GAS constraints. In order to pre-load the NFT, you must: 
- execute the `nft_transfer_call` funtion on the NFT contract and you *must* pass in `pubKey1` (the public key of the keypair created locally and passed into the `send` function) into the `msg` parameter. An example of this can be: 

```bash
near call NFT_CONTRACT.testnet nft_transfer_call '{"token_id": "token1", "receiver_id": "linkdrop-proxy.testnet", "msg": "ed25519:4iwBf6eAXZ4bcN6TWPikSqu3UJ2HUwF8wNNkGZrgDYqE"}' --accountId "benjiman.testnet" --depositYocto 1
```

> **NOTE:** you must send the NFT after the linkdrop has been created. You cannot send an NFT with a public key that isn't on the contract yet.

Once the NFT is sent to the contract, it will be registered and you can view the current information about any key using the `get_key_information` function. Upon claiming, the NFT will be transferred from the contract to the newly created account (or existing account) along with the balance of the linkdrop. If any part of the linkdrop claiming process is unsuccessful, **both** the NFT and the $NEAR will be refunded to the funder and token sender respectively.

> **NOTE:** If the NFT fails to transfer from the contract back to the token sender due to a refund for any reason, the NFT will remain on the proxy contract.

If the linkdrop is successfully claimed, the funder will be refunded for everything **except** the burnt GAS and linkdrop balance. This results in the actual linkdrop cost being extremely low (burnt GAS + initial balance).

For a more in-depth flow-chart, see the [flowcharts](#flowcharts) section.

### Fungible Token Linkdrops

With the proxy contract, users can pre-load a linkdrop with **only one** type of fungible token due to GAS constraints. The number of fungible tokens, however, is not limited. You could load 1 TEAM token, or a million TEAM tokens. You cannot, however, load 10 TEAM tokens and 50 MIKE tokens at the same time.

Due to the nature of how fungible tokens handle storage, the user is responsible for attaching enough $NEAR 
In order to pre-load linkdrop. As mentioned in the [About](#about) section, this amount is dynamically calculated before the linkdrop is created in the `send` function. If you are planning on pre-loading fungible tokens, you must specify the fungible token contract ID when calling the `send` function as shown:

```bash
near call linkdrop-proxy.testnet send '{"public_key": "ed25519:4iwBf6eAXZ4bcN6TWPikSqu3UJ2HUwF8wNNkGZrgDYqE", "balance": "10000000000000000000000", "ft_contract_id": "ft.examples.benjiman.testnet"}' --deposit 1 --accountId "benjiman.testnet"
```

Once all the storage has been paid for, the process for pre-loading the fungible tokens is similar to how you would pre-load an NFT: 

- execute the `ft_transfer_call` function on the FT contract and you *must* pass in `pubKey1` (the public key of the keypair created locally and passed into the `send` function) into the `msg` parameter. An example of this can be: 

```bash
near call FT_CONTRACT.testnet ft_transfer_call '{"receiver_id": "linkdrop-proxy.testnet", "amount": "25", "msg": "ed25519:4iwBf6eAXZ4bcN6TWPikSqu3UJ2HUwF8wNNkGZrgDYqE"}' --accountId "benjiman.testnet" --depositYocto 1
```

> **NOTE:** you must send the FT after the linkdrop has been created. You cannot send an NFT with a public key that isn't on the contract yet. You are also responsible for registering the proxy contract for the given fungible token contract if it isn't registered already.

Once the fungible tokens are sent to the contract, they will be registered and you can view the current information about any key using the `get_key_information` function. Upon claiming, the proxy contract will register the newly created account (or existing account) on the fungible token contract using the storage you depositted in the `send` function. After this is complete, the fungible tokens will be transferred from the contract to the claimed account along with the balance of the linkdrop. If any part of the linkdrop claiming process is unsuccessful, **both** the fungible tokens and the $NEAR will be refunded to the funder and token sender respectively.

> **NOTE:** If the FT fails to transfer from the contract back to the token sender due to a refund for any reason, the fungible tokens will remain on the proxy contract.

If the linkdrop is successfully claimed, the funder will be refunded for everything **except** the burnt GAS, linkdrop balance, and fungible token storage.

For a more in-depth flow-chart, see the [flowcharts](#flowcharts) section.

## Getting Started



### Prerequisites

In order to successfully use this contract, you should have the following installed on your machine: 


- [NEAR account](https://docs.near.org/docs/develop/basics/create-account)
- [rust toolchain](https://docs.near.org/docs/develop/contracts/rust/intro#installing-the-rust-toolchain)
- [NEAR CLI](https://docs.near.org/docs/tools/near-cli#setup)

If you want to run the deploy scripts, you'll need:
- [Node JS](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm)

### Quickstart

The project comes with several useful scripts in order to test and view functionalities for creating linkdrops. These scripts include:

- [simple.js](deploy/simple.js) creating a linkdrop preloaded with just $NEAR
- [nft.js](deploy/nft.js) creating a linkdrop preloaded with $NEAR and an NFT
- [ft.js](deploy/ft.js) creating a linkdrop preloaded with $NEAR and fungible tokens.

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
near call YOUR_CONTRACT_ID.testnet new '{"linkdrop_contract": "testnet"}' --accountId YOUR_CONTRACT_ID.testnet
```

You're now ready to create custom linkdrops pre-loaded with NFTs and FTs. You can either interact with the contract directly using the CLI or use one of the pre-deployed scripts.

### Using the CLI
After the contract is deployed, you have a couple options for creating linkdrops: 

- Creating single linkdrops.
- Creating multiple linkdrops at a time.

This will cover creating single linkdrops, however, the only differences between `send` and `send_multiple` are outlined in the [flowcharts](#flowcharts) section. 

- Start by creating a keypair locally (you can use near-api-js to do this as seen in the deploy scripts).
- Call the `send` function and pass in the `public_key`, `balance`, and if you will be pre-loading fungible tokens, pass in the `ft_contract_id` as well.

```bash
near call YOUR_CONTRACT_ID.testnet send '{"public_key": "ed25519:4iwBf6eAXZ4bcN6TWPikSqu3UJ2HUwF8wNNkGZrgDYqE", "balance": "10000000000000000000000", "ft_contract_id": "ft.examples.benjiman.testnet"}' --deposit 1 --accountId "benjiman.testnet"
```

You must attach enough deposit to cover:
- Desired linkdrop balance (in this case, 0.01 $NEAR)
- 0.02 $NEAR for the function call access key allowance.
- 0.00284 $NEAR for the base cost of creating an account ID (due to storage). This number is based on the largest possible account ID (64 characters).
- 0.005 $NEAR for storing the account details + key on the proxy contract
- *Optional* storage for registering the new account ID on a fungible token contract. This amount is dependant on the FT contract and is dynamically calculated before a linkdrop is created (in the send function).

Once the function is successful, you can create the link and click it to claim the linkdrop:
```
    wallet.testnet.near.org/linkdrop/{YOUR_CONTRACT_ID.testnet}/{privKey1}
```

### Using the pre-deployed scripts

If you'd like to use some of the deploy scripts found in the `deploy` folder, those can help automate the process. 

#### Simple Linkdrops with No NFTs or FTs

If you'd like to create a simple linkdrop with no pre-loaded NFTs or FTs, first specify the following environment variables:

```bash
export LINKDROP_PROXY_CONTRACT_ID="INSERT_HERE"
export FUNDING_ACCOUNT_ID="INSERT_HERE"
export LINKDROP_NEAR_AMOUNT="INSERT_HERE"
```

This will set the proxy contract that you wish to create linkdrops on, the account ID of the funding address (person creating the linkdrops and sending the funds), and the actual $NEAR amount that the linkdrop will contain. It is recommended to simply run a `dev-deploy` and use the dev contract ID to test these scripts. Once this is finished, run the following script:

```
node deploy/simple.js
```

Once the script has finished executing, a link to the wallet should appear in your console similar to: 

```bash
https://wallet.testnet.near.org/linkdrop/dev-1652794689263-24159113353222/4YULUt1hqv4s96Z8K83VoPnWqXK9vjfYb5QsBrv793aZ2jucBiLP35YWJq9rPGziRpDM35HEUftUtpP1WLzFocqJ
```

Once you've clicked the link, you can either fund an existing account with the linkdrop balance, or you can create a new account and fund it that way.

#### Linkdrops with NFTs

If you'd like to create a linkdrop with a pre-loaded NFT, first specify the following environment variables:

```bash
export LINKDROP_PROXY_CONTRACT_ID="INSERT_HERE"
export FUNDING_ACCOUNT_ID="INSERT_HERE"
export LINKDROP_NEAR_AMOUNT="INSERT_HERE"
```

If you ran the script now, it would mint a predefined NFT on the contract `example-nft.testnet`. If you wish to change the NFT contract or the metadata for the token, simply open the `deploy/nft.js` script and change the following lines:

```js
/*
	Hard coding NFT contract and metadata. Change this if you want.
*/
let NFT_CONTRACT_ID = "example-nft.testnet";
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

<img src="assets/claimed-nft.png" alt="Logo">

#### Linkdrops with FTs

If you'd like to create a linkdrop with some pre-loaded FTs, you'll need to first specify the following environment variables:

```bash
export LINKDROP_PROXY_CONTRACT_ID="INSERT_HERE"
export FUNDING_ACCOUNT_ID="INSERT_HERE"
export LINKDROP_NEAR_AMOUNT="INSERT_HERE"
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

<img src="assets/claimed-ft.png" alt="Logo">

## Flowcharts

### Creating Single Linkdrops
<img src="flowcharts/creating-single-linkdrops.png" alt="Logo">


### Creating Multiple Linkdrops
<img src="flowcharts/creating-multiple-linkdrops.png" alt="Logo">

### Adding NFTs and FTs to Linkdrops
<img src="flowcharts/adding-nfts-and-fts-to-linkdrops.png" alt="Logo">

### Claiming NFT Linkdrops With New Accounts
<img src="flowcharts/claiming-nft-linkdrops-with-new-accounts.png" alt="Logo">

### Claiming FT Linkdrops With New Accounts
<img src="flowcharts/claiming-ft-linkdrops-with-new-accounts.png" alt="Logo">




## Contributing

First off, thanks for taking the time to contribute! Contributions are what makes the open-source community such an amazing place to learn, inspire, and create. Any contributions you make will benefit everybody else and are **greatly appreciated**.

Please try to create bug reports that are:

- _Reproducible._ Include steps to reproduce the problem.
- _Specific._ Include as much detail as possible: which version, what environment, etc.
- _Unique._ Do not duplicate existing opened issues.
- _Scoped to a Single Bug._ One bug per report.

Please adhere to this project's [code of conduct](docs/CODE_OF_CONDUCT.md).

You can use [markdownlint-cli](https://github.com/igorshubovych/markdownlint-cli) to check for common markdown style inconsistency.

## License

This project is licensed under the **GPL License**.

## Acknowledgements

Thanks for these awesome resources that were used during the development of the **Linkdrop Proxy Contract**:

- <https://github.com/dec0dOS/amazing-github-template>
- <https://github.com/near/near-linkdrop>
- <https://github.com/near/near-wallet/blob/master/packages/frontend/docs/Linkdrop.md>
