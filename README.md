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
2. Companies don't want to expose full accessnearcon-beta-keypom-nfts.near keys in their backend servers.
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

## NFT Drops

With Keypom contract, users can pre-load each key with a set of NFTs depending on how many uses per key. Each use will pop the last token ID off and send it to the claimed account.

In order to pre-load NFTs and register a key use, you must:
- Add enough $NEAR to your account balance
- create a drop and specify NFTData for the NFTs that will be sent to the contract. 
- Add a key to the drop
- Send an NFT with a token ID shorter than the longest token ID specified

An example of creating an NFT drop can be seen:

```bash
near call keypom.testnet create_drop 
'{
  "public_keys": [
    "ed25519:4Adq6WiKVjGz56Ena6D1w2UnADuZpiFBWAz12cfnkibv"
  ],
  "deposit_per_use": "5000000000000000000000",
  "nft_data": {
    "contract_id": "nft.examples.testnet",
    "sender_id": "benjiman.testnet",
    "longest_token_id": "ed25519:4Adq6WiKVjGz56Ena6D1w2UnADuZpiFBWAz12cfnkibv"
  },
  "config": {
    "uses_per_key": 2,
  },
  "metadata": "{\"title\":\"This is a title\",\"description\":\"This is a description\"}"
}' 
--accountId "benjiman.testnet"
```

- Once the regular drop has been created with at least 1 key, execute the `nft_transfer_call` function on the NFT contract and you *must* pass in the drop ID into the `msg` parameter. The token ID will then be pushed to the end of the list of registered token IDs for the drop. 

```bash
near call nft.examples.testnet nft_transfer_call '{"token_id": "token1", "receiver_id": "keypom.testnet", "msg": "1"}' --accountId "benjiman.testnet" --depositYocto 1
```

> **NOTE:** you must send the NFT after the drop has been created with at least 1 key. If you send more NFTs than the number of uses left, the NFT will be kept by the contract.

Once the NFT is sent to the contract, it will be registered and you can view the current information about any key using the `get_key_information` function. Upon claiming, the NFT will be transferred from the contract to the newly created account (or existing account) along with the balance of the key. If any part of the key claiming process is unsuccessful, **both** the NFT and the $NEAR will be refunded to the funder and token sender respectively.

> **NOTE:** If the NFT fails to transfer from the contract back to the token sender due to a refund for any reason, the NFT will remain on the Keypom.

## Fungible Token Drops

With Keypom, users can pre-load a drop with **only one** type of fungible token due to GAS constraints. The number of fungible tokens, however, is not limited. You could load 1 TEAM token, or a million TEAM tokens. You cannot, however, load 10 TEAM tokens and 50 MIKE tokens at the same time.

Due to the nature of how fungible token contracts handle storage, the user is responsible for attaching enough $NEAR to cover the registration fee. As mentioned in the [About](#about) section, this amount is dynamically calculated before the drop is created in the `create_drop` function. The process for creating fungible token drop is very similar to NFTs:
- Add enough $NEAR to your account balance
- create a drop and specify FTData for the Fungible Tokens that will be sent to the contract. 
- Add a key to the drop
- Send the FTs and pass in the drop ID


An example of creating an FT drop can be seen:

```bash
export LINKDROP_PROXY_CONTRACT_ID="INSERT_HERE"
export FUNDING_ACCOUNT_ID="INSERT_HERE"
export LINKDROP_NEAR_AMOUNT="INSERT_HERE"
export SEND_MULTIPLE="false"
```

Once the drop is created with the fungible token data, you can the send the fungible tokens to register uses.
- execute the `ft_transfer_call` function on the FT contract and you *must* pass in the drop ID into the `msg` parameter. An example of this can be: 

```bash
near call FT_CONTRACT.testnet ft_transfer_call 
`{
  "receiver_id": "keypom.testnet",
  "amount": "25",
  "msg": "0"
}`
--accountId "benjiman.testnet" --depositYocto 1
```

> **NOTE:** you must send the FTs after the drop has been created with at least 1 key. If you send more FTs than the number of uses left, the FTs will be kept by the contract. You are also responsible for registering the Keypom contract for the given fungible token contract if it isn't registered already.

Once the fungible tokens are sent to the contract, they will be registered and you can view the current information about any key using the `get_key_information` function. Upon claiming, the contract will register the newly created account (or existing account) on the fungible token contract using the storage you deposited. After this is complete, the fungible tokens will be transferred from the contract to the claimed account along with the balance of the key. If any part of the key claiming process is unsuccessful, **both** the fungible tokens and the $NEAR will be refunded to the funder and token sender respectively.

> **NOTE:** If the FT fails to transfer from the contract back to the token sender due to a refund for any reason, the fungible tokens will remain on the proxy contract.

## Function Calls

Let's look at an example to see the power of the keypom contract. If a user wants to be able to lazy mint two NFTs everytime a key is used but the mint function takes a parameter `receiver_id` and a deposit of 1 $NEAR, you could specify these parameters.

If there was a different NFT contract where the parameter was `nft_contract_id` instead, that is also possible. You can specify the exact field that the claiming account ID should be passed into. An example flow of creating a function call drop is below.

- create a drop and specify the FC (function call) data for the function that will be called upon claim

```bash
near call keypom.testnet send 
'{
  "public_keys": [
    "ed25519:3ANjBcTh6ZNTBqj9KLdTxXtW7ChnuSfc6n4rJMzkXrE9",
  ],
  "deposit_per_use": "5000000000000000000000",
  "fc_data": {
    "methods": [
      [
        {
          "receiver_id": "nft.examples.testnet",
          "method_name": "nft_mint",
          "args": "{\"token_id\":\"test-one\",\"metadata\":{\"title\":\"Linkdropped Go Team NFT\",\"description\":\"Testing Linkdrop NFT Go Team Token\",\"media\":\"https://bafybeiftczwrtyr3k7a2k4vutd3amkwsmaqyhrdzlhvpt33dyjivufqusq.ipfs.dweb.link/goteam-gif.gif\",\"media_hash\":null,\"copies\":10000,\"issued_at\":null,\"expires_at\":null,\"starts_at\":null,\"updated_at\":null,\"extra\":null,\"reference\":null,\"reference_hash\":null}}",
          "attached_deposit": "1000000000000000000000000"
        },
        {
          "receiver_id": "nft.examples.testnet",
          "method_name": "nft_mint",
          "args": "{\"token_id\":\"test-two\",\"metadata\":{\"title\":\"Linkdropped Go Team NFT\",\"description\":\"Testing Linkdrop NFT Go Team Token\",\"media\":\"https://bafybeiftczwrtyr3k7a2k4vutd3amkwsmaqyhrdzlhvpt33dyjivufqusq.ipfs.dweb.link/goteam-gif.gif\",\"media_hash\":null,\"copies\":10000,\"issued_at\":null,\"expires_at\":null,\"starts_at\":null,\"updated_at\":null,\"extra\":null,\"reference\":null,\"reference_hash\":null}}",
          "attached_deposit": "1000000000000000000000000"
        }
      ]
    ],
    "config": {
      "account_id_field": "receiver_id",
      "drop_id_field": "custom_drop_id",
      "key_id_field": "custom_key_id"
    }
  },
  "config": {
    "uses_per_key": 2,
  },
  "metadata": "{\"title\":\"This is a title\",\"description\":\"This is a description\"}"
}'
--accountId "benjiman.testnet"
```
This will create a drop with 1 key that can be used 2 times. Everytime the key is used, it will call the `nft_mint` function on the NFT contract. The first time it will mint a token with the token ID `test-one` and the second time it will mint a token with the token ID `test-two`. In addition, the account Id, drop Id, and key Id fields will be sent in the arguments.

# Getting Started

## Query Information

Keypom allows users to query a suite of different information from the contract. This information can be broken down into two separate objects that are returned. JsonDrops and JsonKeys.
```rs
pub struct JsonDrop {
    // Drop ID for this drop
    pub drop_id: DropId,
    // owner of this specific drop
    pub owner_id: AccountId,
    // Balance for all keys of this drop. Can be 0 if specified.
    pub deposit_per_use: U128,
    // Every drop must have a type
    pub drop_type: JsonDropType,
    // The drop as a whole can have a config as well
    pub config: Option<DropConfig>,
    // Metadata for the drop
    pub metadata: Option<DropMetadata>,
    // How many claims
    pub registered_uses: u64,
    // Ensure this drop can only be used when the function has the required gas to attach
    pub required_gas: Gas,
    // Keep track of the next nonce to give out to a key
    pub next_key_id: u64,
}

pub struct JsonKeyInfo {
    // Drop ID for the specific drop
    pub drop_id: DropId,
    pub pk: PublicKey,
    pub key_info: KeyInfo {
      // How many uses this key has left. Once 0 is reached, the key is deleted
      pub remaining_uses: u64,
      // When was the last time the key was used
      pub last_used: u64,
      // How much allowance does the key have left. When the key is deleted, this is refunded to the funder's balance.
      pub allowance: u128,
      // Nonce for the current key.
      pub key_id: u64,
    },
}
```

### Key Specific
- **`get_key_balance(key: PublicKey)`**: Returns the $NEAR that will be sent to the claiming account when the key is used 
- **`get_key_total_supply()`**: Returns the total number of keys currently on the contract
- **`get_keys(from_index: Option<U128>, limit: Option<u64>)`**: Paginate through all keys on the contract and return a vector of key info
- **`get_key_information(key: PublicKey)`**: Return the key info for a specific key

### Drop Specific
- **`get_drop_information(drop_id: Option<DropId>, key: Option<PublicKey>)`**: Return the drop info for a specific drop. This can be queried for by either passing in the drop ID or a public key.
- **`get_key_supply_for_drop(drop_id: DropId)`**: Return the total number of keys for a specific drop
- **`get_keys_for_drop(drop_id: DropId, from_index: Option<U128>, limit: Option<u64>)`**: Paginate through all keys for a specific drop and return a vector of key info
- **`get_drop_supply_for_owner(account_id: AccountId)`**: Return the total number of drops for a specific account
- **`get_drops_for_owner(account_id: AccountId, from_index: Option<U128>, limit: Option<u64>)`**: Paginate through all drops for a specific account and return a vector of drop info 
- **`get_nft_supply_for_drop(drop_id: DropId)`**: Get the total number of NFTs registered for a given drop.
- **`get_nft_token_ids_for_drop(drop_id: DropId, from_index: Option<U128>, limit: Option<u64>)`**: Paginate through token IDs for a given drop
- **`get_next_drop_id()`**: Get the next drop ID that will be used for a new drop

### Utility
- **`get_root_account()`**: Get the global root account that all created accounts with be based off.
- **`get_user_balance()`**: Get the current user balance for a specific account.


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

Thanks for these awesome resources that were used during the development of the **Keypom Contract**:

- <https://github.com/dec0dOS/amazing-github-template>
- <https://github.com/near/near-linkdrop>
- <https://github.com/near/near-wallet/blob/master/packages/frontend/docs/Linkdrop.md>
