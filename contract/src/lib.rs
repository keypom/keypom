/*!
Keypom is an access key factory created as a result of 3 common problems that arose in the ecosystem.

1. People want a *cheap, customizable, and unique* onboarding experience for users.
2. Companies don't want to expose **full access keys** in their backend servers.
3. dApps want a *smooth UX* for interactions that require deposits.

To solve this, Keypom allows for the creation of highly customizable access keys. Each access key
has a different functionality depending on which type of *drop* they derive from. A drop can be thought
of as a bucket that access keys can be part of. An owner will create drops of a certain type
with a set of features that all the keys within it will derive from. A drop can be one of four
different types:

1. Simple drops
2. Non Fungible Token drops
3. Fungible Token drops
4. Function Call drops.

Once a drop has been created, all keys added will share the same behaviour as outlined by the type of drop 
and the configurations present. These keys can be used to either claim with:
- An **existing** NEAR account through the `claim` function.
- A new account that doesn't exist yet is created through the `create_account_and_claim` function.

# Shared Drop Customization

While each *type* of drop has its own set of customizable features, there are some that are shared by **all drops** 
These are outlined below.

```rust
/// Each time a key is used, how much $NEAR should be sent to the claiming account (can be 0).
pub deposit_per_use: u128,

/// How much Gas should be attached when the key is used. The default is 100 TGas as this is what's used by the NEAR wallet.
pub required_gas: Gas,

/// The drop as a whole can have a config as well
pub config: Option<DropConfig>,

/// Metadata for the drop in the form of stringified JSON. The format is completely up to the user and there are no standards for format.
pub metadata: LazyOption<DropMetadata>,
```

Within the config, there are a suite of features that can be customized as well.

```rust
/// How many uses can each key have before it's deleted. If None, default to 1.
pub uses_per_key: Option<u64>,

/// Minimum block timestamp before keys can be used. If None, keys can be used immediately
/// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
pub start_timestamp: Option<u64>,

/// How often can a key be used. This specifies the time between each use.
/// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
pub throttle_timestamp: Option<u64>,

/// If claim is called, refund the `deposit_per_use` to the owner's account directly. If None, default to false.
pub on_claim_refund_deposit: Option<bool>,

/// What permissions does the key have? Can it call both `claim` and `create_account_and_claim` or just one of the two?
/// This defaults to the key being able to call both methods.
pub claim_permission: Option<ClaimPermissions>,

/// Override the global root account that sub-accounts will have (near or testnet). This allows users to create
/// Specific drops that can create sub-accounts of a predefined root. For example, fayyr could specify a root of `fayyr.near`
/// By which all sub-accounts will then be `ACCOUNT.fayyr.near`
pub drop_root: Option<AccountId>,
```

# Simple Drops

The most basic type of drop is the simple kind. Any keys that are part of a simple drop can
only be used for 1 thing: **transferring $NEAR**. Once the key is claimed, the claiming account
will receive the $NEAR specified in the `deposit_per_use`. Simple drops are a great way to send $NEAR to claiming accounts while not storing a lot
of information on the contract. Below are a couple use cases.

#### Backend Servers

Let's say you have a backend server that should send 10 $NEAR to the first 3
people that redeem an NFT. Rather than exposing your full access key in the backend server, 
you could create a simple drop that either has 3 keys or 1 key that is claimable 3 times.
In the drop, you'd specify that each time the key is claimed, the specified account would
receive 10 $NEAR.

#### Recurring Payments

Recurring payments are quite a common situation. If you need to send someone 10 $NEAR once a
month for 6 months, you could create a simple drop that has a `throttle_timestamp` of 1 month
in between uses and everytime it's used, 10 $NEAR is sent to the account. In addition, you
could specify a `start_timestamp` to determine the date at which the key can first be used.

#### Quick Onboarding

If you need to quickly onboard users onto NEAR, you could create a simple drop with a 
small amount of $NEAR (enough to create a wallet) and set the claim permission to be 
`CreateAccountAndClaim`. This means that the key can only be used to create accounts.
You can then add keys as you wish to the drop and give them out to users so they can create
accounts and be onboarded onto NEAR.

# Non-Fungible Token Drops

Non-Fungible Token drops are a special type that allows users to "preload" the drop with NFTs. 
These tokens will then be *automatically* sent to the **claiming user**. The claiming flow
is fairly similar to simple drops in that users can either create an account or claim to an
existing one.

NFT drops are essentially a wrapper around simple drops. All the functionalities that simple 
drops have are carried over but now, users can receive an NFT as well as $NEAR. This brings 
introduces some customization and uniqueness to the use-cases.

## How does it work?

Every drop has a field known as `registered_uses`. This tells the contract how many uses the
drop has across all its keys. For simple drops, this field doesn't matter since all the uses
are paid for up-front when the drop is created or when keys are added. With NFT drops, however,
there is a 2 step process:
- Firstly, the drop is created and all the $NEAR required is pre-paid for. This is the same as
simple drops, however, the `registered_uses` are set to 0.
- Once the drop is created, the owner must send the contract the NFTs in order for keys to be
usable. This process is done through the `nft_transfer_call` workflow baked into the NFT standards.
It's up to the owner to facilitate this process.

:::info
It's important to note that the drop owner must specify the **longest token ID** and prepay for the
storage costs of storing these token IDs. When the NFT is transferred, the contract will ensure
that the incoming token ID is smaller than the longest one specified.
:::

Whenever the contract receives tokens, it will push the ID to a vector. These IDs are **popped** off
whenever a key is used. A user will receive the most recent token sent to the contract as the
vector is acting like a *stack*.

## Use Cases

NFT drops work really well for when you want to send a *pre-existing* NFT to a user along with
some $NEAR. Since NFT drops are a light wrapper around simple drops, most of the use-cases are
the same although people can now get NFTs as well. This means you can onboard a user with some
$NEAR **and** they *get an NFT* too.

## NFT Config

Along with the default global configurations for drops, if you'd like to create an NFT drop,
you must specify the following pieces of information when the drop is created.

```rust
pub struct NFTDataConfig {
    /// Which account ID will be sending the NFTs to the contract
    pub sender_id: AccountId,
    /// Which contract will the NFTs live on
    pub contract_id: AccountId,
    /// What will be the longest possible token ID that will be sent to the contract
    /// for this specific drop
    pub longest_token_id: String,
}
```
 
By specifying this information, the drop is locked into only accepting NFTs from the sender, contract and
that have a token ID less than the longest one specified.

# Fungible Token Drops

A Fungible Token drop is also a light wrapper around the simple drop. It works very similarly to how its NFT
counterpart does. First, you'll need to create the drop and then you can fund it with assets and register
key uses. The only difference between NFT and FT drops is that for FTs, you can over-register assets.

You can preload a drop with as many FTs as you'd like even if you don't have the keys yet. This will spike the 
`registered_uses` and then you can create keys and slowly eat away from this "total supply" overtime. If the
drop runs out, you can send it more FTs to top up. All the keys in the FT drop will share from this supply
and everytime a key is used, the `registered_uses` will decrement and the "total supply" will get smaller.

## How does it work?

As mentioned in the NFT section, every drop has a field known as `registered_uses`. This tells the contract 
how many uses the drop has across all its keys. For simple drops, this field doesn't matter since all the uses
are paid for up-front when the drop is created or when keys are added. With FT drops, however,
there is a 2 step process:
- Firstly, the drop is created and all the $NEAR required is pre-paid for. This is the same as
simple drops, however, the `registered_uses` are set to 0.
- Once the drop is created, the owner must send the contract the FTs in order for keys to be
usable. This process is done through the `ft_transfer_call` workflow baked into the FT standards.
It's up to the owner to facilitate this process.

## Use Cases

FT drops work really due to the fact that they support all the functionalities of the Simple drops, just with
more use-cases and possibilities. Let's look at some use cases to see how fungible token drops can be used.

#### Recurring Payments

Recurring payments are quite a common situation. Let's say you need to send someone $50 USDC every week. You
could create a key with 5 claims that has a throttle_timestamp` of 1 week. You would then pre-load maybe the
first week's deposit of $50 USDC and register 1 use or you could send $500 USDC for the first 10 weeks. At that
point, you would simply hand over the key to the user and they can claim once a week.

#### Backend Servers

Taking the recurring payments problem to another level, imagine that instead of leaving the claims up to the
person, you wanted to automatically pay them through a backend server. They would give you their NEAR account
and you would send them FTs. The problem is that you don't want to expose your full access key in the server.
By creating a FT drop, you can store **only the function call access key** created by Keypom in the server.
Your backend would them use the key to call the `claim` function and pass in the user's account ID to send
them the FTs.

#### Creating a Wallet with FTs

Another awesome use-case is to allow users to be onboarded onto NEAR and **also** receive FTs. As an example, 
You could do a promotion where you're giving away $10 USDC to the first 100 users that sign up to your mailing 
list. You can also give away QR codes at events that contain a new fungible token that you're launching. You can
simply create a FT drop and pre-load it with the FT of your choice. In addition, you can give it 0.02 $NEAR for
new wallets that are created.

You can pair this with setting the `on_claim_refund_deposit` flag to true which would make it so that if anyone claims 
the fungible tokens and they *already have a wallet*, it will automatically refund you the 0.02 $NEAR. That money should
only be used for the creation of new wallets. Since your focus is on the fungible tokens, you don't want to **force users**
to create a new wallet if they have one already by specifying the claim permission to be `CreateAccountAndClaim` but instead,
you want to be refunded in case they do.

## FT Config

Along with the default global configurations for drops, if you'd like to create a FT drop,
you must specify the following pieces of information when the drop is created.

```rust
pub struct FTDataConfig {
    /// The contract that the FTs live on.
    pub contract_id: AccountId,
    /// The account ID that will be sending the FTs to the contract.
    pub sender_id: AccountId,
    /// How many FTs should the contract send *each time* a key is used.
    pub balance_per_use: U128,
}
```
 
By specifying this information, the drop is locked into only accepting FTs coming from the sender and contract. While
you can send as many FTs as you'd like and can over-pay, you *must* send at **least** enough FTs in one call to cover
1 use. As an example, if a drop is created such that 10 FTs will be sent when a key is used, you must send **at least 10**
and cannot break it up into separate calls where you send 5 one time and 5 another.
!*/


#![warn(missing_docs)]

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk::{
    env, ext_contract, near_bindgen, promise_result_as_success, require, AccountId, Balance,
    BorshStorageKey, CryptoHash, Gas, PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
    PublicKey,
};

/*
    minimum amount of storage required to store an access key on the contract
*/
const ACCESS_KEY_STORAGE: u128 = 1_000_000_000_000_000_000_000; // 0.001 N

/*
    minimum amount of NEAR that a new account (with longest possible name) must have when created
    If this is less, it will throw a lack balance for state error (assuming you have the same account ID length)
*/
const NEW_ACCOUNT_BASE: u128 = 2_840_000_000_000_000_000_000; // 0.00284 N

/// Indicates there are no attached_deposit for a callback for better readability.
const NO_DEPOSIT: u128 = 0;

/*
    GAS Constants (outlines the minimum to attach. Any unspent GAS will be added according to the weights)
*/
const MIN_GAS_FOR_ON_CLAIM: Gas = Gas(55_000_000_000_000); // 55 TGas

// NFTs
const MIN_GAS_FOR_SIMPLE_NFT_TRANSFER: Gas = Gas(10_000_000_000_000); // 10 TGas
const MIN_GAS_FOR_RESOLVE_TRANSFER: Gas =
    Gas(15_000_000_000_000 + MIN_GAS_FOR_SIMPLE_NFT_TRANSFER.0); // 15 TGas + 10 TGas = 25 TGas

// FTs
// Actual amount of GAS to attach when querying the storage balance bounds. No unspent GAS will be attached on top of this (weight of 0)
const GAS_FOR_STORAGE_BALANCE_BOUNDS: Gas = Gas(10_000_000_000_000); // 10 TGas
const MIN_GAS_FOR_RESOLVE_STORAGE_CHECK: Gas = Gas(25_000_000_000_000); // 25 TGas
const MIN_GAS_FOR_FT_TRANSFER: Gas = Gas(5_000_000_000_000); // 5 TGas
const MIN_GAS_FOR_STORAGE_DEPOSIT: Gas = Gas(5_000_000_000_000); // 5 TGas
const MIN_GAS_FOR_RESOLVE_BATCH: Gas =
    Gas(13_000_000_000_000 + MIN_GAS_FOR_FT_TRANSFER.0 + MIN_GAS_FOR_STORAGE_DEPOSIT.0); // 13 TGas + 5 TGas + 5 TGas = 23 TGas

// Specifies the GAS being attached from the wallet site
const ATTACHED_GAS_FROM_WALLET: Gas = Gas(100_000_000_000_000); // 100 TGas

// Specifies the amount of GAS to attach on top of the FC Gas if executing a regular function call in claim
const GAS_OFFSET_IF_FC_EXECUTE: Gas = Gas(20_000_000_000_000); // 20 TGas

// Actual amount of GAS to attach when creating a new account. No unspent GAS will be attached on top of this (weight of 0)
const GAS_FOR_CREATE_ACCOUNT: Gas = Gas(28_000_000_000_000); // 28 TGas

/// Both methods callable by the function call access key
const ACCESS_KEY_BOTH_METHOD_NAMES: &str = "claim,create_account_and_claim";

/// Only the claim method_name is callable by the access key
const ACCESS_KEY_CLAIM_METHOD_NAME: &str = "claim";

/// Only the create_account_and_claim method_name is callable by the access key
const ACCESS_KEY_CREATE_ACCOUNT_METHOD_NAME: &str = "create_account_and_claim";

/*
    FEES
*/
const DROP_CREATION_FEE: u128 = 0; //1_000_000_000_000_000_000_000_000; // 1 N
const KEY_ADDITION_FEE: u128 = 0; //5_000_000_000_000_000_000_000; // 0.005 N

const GAS_FOR_PANIC_OFFSET: Gas = Gas(10_000_000_000_000); // 10 TGas

mod internals;
mod stage1;
mod stage2;
mod stage3;
mod views;

use internals::*;
use stage1::*;
use stage2::*;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    DropIdForPk,
    DropsForId,
    DropIdsForFunder,
    DropIdsForFunderInner { account_id_hash: CryptoHash },
    PksForDrop { account_id_hash: CryptoHash },
    DropMetadata { account_id_hash: CryptoHash },
    TokenIdsForDrop { account_id_hash: CryptoHash },
    FeesPerUser,
    UserBalances,
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
pub struct Keypom {
    /// THIS IS A TEST
    pub owner_id: AccountId,
    // Which contract is the actual linkdrop deployed to (i.e `testnet` or `near`)
    pub root_account: AccountId,

    // Map each key to a nonce rather than repeating each drop data in memory
    pub drop_id_for_pk: UnorderedMap<PublicKey, DropId>,
    // Map the nonce to a specific drop
    pub drop_for_id: LookupMap<DropId, Drop>,
    // Keep track of the drop ids for each funder for pagination
    pub drop_ids_for_owner: LookupMap<AccountId, UnorderedSet<DropId>>,

    // Fees taken by the contract. One is for creating a drop, the other is for each key in the drop.
    pub drop_fee: u128,
    pub key_fee: u128,
    pub fees_collected: u128,

    // Keep track of fees per each user. Only the owner can edit this.
    pub fees_per_user: LookupMap<AccountId, (u128, u128)>,

    // keep track of the balances for each user. This is to prepay for drop creations
    pub user_balances: LookupMap<AccountId, Balance>,

    // Keep track of a nonce used for the drop IDs
    pub next_drop_id: DropId,

    // Keep track of the price of 1 GAS per 1 yocto
    pub yocto_per_gas: u128,
}

#[near_bindgen]
impl Keypom {
    /// Initialize contract and pass in the desired deployed linkdrop contract (i.e testnet or near)
    #[init]
    pub fn new(root_account: AccountId, owner_id: AccountId) -> Self {
        Self {
            owner_id,
            root_account,
            drop_id_for_pk: UnorderedMap::new(StorageKey::DropIdForPk),
            drop_for_id: LookupMap::new(StorageKey::DropsForId),
            drop_ids_for_owner: LookupMap::new(StorageKey::DropIdsForFunder),
            user_balances: LookupMap::new(StorageKey::UserBalances),
            next_drop_id: 0,
            /*
                FEES
            */
            fees_per_user: LookupMap::new(StorageKey::FeesPerUser),
            drop_fee: DROP_CREATION_FEE,
            key_fee: KEY_ADDITION_FEE,
            fees_collected: 0,
            yocto_per_gas: 100_000_000,
        }
    }
}
