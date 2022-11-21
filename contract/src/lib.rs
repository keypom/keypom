/*!
# Introduction

Keypom is an access key factory created as a result of 3 common problems that arose in the ecosystem.

1. People want a *cheap, customizable, and unique* onboarding experience for users.
2. Companies don't want to expose **full access keys** in their backend servers.
3. dApps want a *smooth UX* with zero barrier to entry onboarding.

> To view our debut talk at NEARCON 2022, click [here](https://www.youtube.com/watch?v=J-BOnfhHV50).

Blockchain technology comes with many benefits such as sovereign ownership, digital rights, privacy, freedom,
peer to peer coordination and much more. The problem with this technology, however, is that there is an extremely
high barrier to entry for an everyday individual. None of it matters if nobody can onboard.

It’s confusing to create and fund a crypto wallet. People are unfamiliar with the process, technical jargon,
and the general flow. NEAR’s account model is powerful, but extremely underutilized because it’s complex for
developers to take full advantage of. Keypom wraps this up in a single API call.

With NEAR’s goal of onboarding 1 billion users to Web3, there needs to be a solution to this high barrier to
entry for developers building on NEAR and users onboarding to their apps and the NEAR ecosystem.

Below is a table outlining the minimum costs to onboard a new user onto NEAR with a named account.

|                      | 1 Account       | 1,000 Accounts  | 1,000,000 Accounts |
|----------------------|-----------------|-----------------|--------------------|
| Traditional Linkdrop | ~1 NEAR         | ~1,003 NEAR     | ~1,002,840 NEAR    |
| Keypom               | ~0.0035 NEAR    | ~3.5 NEAR       | ~3,500 NEAR        |
|                      | ~99.65% Cheaper | ~99.65% Cheaper | ~99.65% Cheaper    |

Keypom allows anyone to create highly customizable onboarding experiences for their users. These experiences
can be both for new, or existing users. If someone already has a wallet, they can still use a Keypom link to
experience an app, and then transfer the assets later.

## Comparable Solutions

|                                              | **Keypom** | **NEAR Drop** | **Satori** |
|----------------------------------------------|------------|---------------|------------|
| NEAR Drop                                    |      ✅     |       ✅       |      ❌     |
| FT Drop                                      |      ✅     |       ❌       |      ❌     |
| NFT Drop                                     |      ✅     |       ❌       |      ✅     |
| Function Call Drop                           |      ✅     |       ❌       |      ❌     |
| Embeddable in Dapps                          |      ✅     |       ❌       |      ❌     |
| Wallet Selector Integration                  |      ✅     |       ❌       |      ❌     |
| No Fee                                       |      ✅     |     Maybe?    |      ❌     |
| No Backend / 3rd Party                       |      ✅     |       ✅       |      ❌     |
| Campaigns                                    |      ✅     |       ✅       |      ✅     |
| Multi-Step e.g. Tickets click > scan > claim |      ✅     |       ❌       |      ❌     |
| Password Protected Drops                     |      ✅     |       ❌       |      ❌     |
| Timed Drops e.g. recurring payments          |      ✅     |       ❌       |      ❌     |
| Custom Names e.g. user.myapp.near            |      ✅     |       ❌       |      ❌     |

# Our Solution

Keypom allows for the creation of highly customizable access keys. These keys can be thought of as having their
own *smart contracts*. Each access key derives from what's known as a *drop*. These drops outline the different
functionalities and behaviors the key will have. A drop can be thought of as a bucket that access keys belong to.
You can create many different buckets and fill them each with their own keys. Each key will act in accordance to the
drop, or bucket, it belongs to.

A drop can be one of four different types:

1. Simple drop.
2. Non Fungible Token drop.
3. Fungible Token drop.
4. Function Call drop.

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

/// How much Gas should be attached when the key is used. The default is 100 TGas as this is
/// what's used by the NEAR wallet.
pub required_gas: Gas,

/// The drop as a whole can have a config as well
pub config: Option<DropConfig>,

/// Metadata for the drop in the form of stringified JSON. The format is completely up to the
/// user and there are no standards for format.
pub metadata: LazyOption<DropMetadata>,
```

Within the config, there are a suite of features that can be customized as well:

```rust
/// How many uses can each key have before it's deleted. If None, default to 1.
pub uses_per_key: Option<u64>,

/// Override the global root account that sub-accounts will have (near or testnet). This allows
/// users to create specific drops that can create sub-accounts of a predefined root.
/// For example, Fayyr could specify a root of `fayyr.near` By which all sub-accounts will then
/// be `ACCOUNT.fayyr.near`
pub root_account_id: Option<AccountId>,

// Any time based configurations
pub time: Option<TimeConfig>,

// Any usage specific configurations
pub usage: Option<UsageConfig>,
```

## Time Based Customizations

Keypom allows users to customize time-based configurations as outlined below.

```rust
pub struct TimeConfig {
    /// Minimum block timestamp before keys can be used. If None, keys can be used immediately
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub start: Option<u64>,

    /// Block timestamp that keys must be before. If None, keys can be used indefinitely
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub end: Option<u64>,

    /// Time interval between each key use. If None, there is no delay between key uses.
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub throttle: Option<u64>,

    /// Interval of time after the `start_timestamp` that must pass before a key can be used.
    /// If multiple intervals pass, the key can be used multiple times. This has nothing to do
    /// With the throttle timestamp. It only pertains to the start timestamp and the current
    /// timestamp. The last_used timestamp is not taken into account.
    /// Measured in number of non-leap-nanoseconds since January 1, 1970 0:00:00 UTC.
    pub interval: Option<u64>,
}
```

## Usage Based Customizations

In addition to time-based configurations, the funder can customize behaviors pertaining to
key usages.

```rust
pub struct UsageConfig {
    /// Can the access key only call the claim method_name? Default to both method_name callable
    pub permissions: Option<ClaimPermissions>,
    /// If claim is called, refund the deposit to the owner's balance. If None, default to false.
    pub refund_deposit: Option<bool>,
    /// Should the drop be automatically deleted when all the keys are used? This is defaulted to false and
    /// Must be overwritten
    pub auto_delete_drop: Option<bool>,
    /// When this drop is deleted and it is the owner's *last* drop, automatically withdraw their balance.
    pub auto_withdraw: Option<bool>,
}
```

# Simple Drops

The most basic type of drop is the simple kind. Any keys that are part of a simple drop can
only be used for 1 thing: **transferring $NEAR**. Once the key is claimed, the claiming account
will receive the $NEAR specified in the `deposit_per_use`. Simple drops are a great way to send 
$NEAR to claiming accounts while not storing a lot of information on the contract. Below are a 
couple use cases.

#### Backend Servers

Let's say you have a backend server that should send 10 $NEAR to the first 3
people that redeem an NFT. Rather than exposing your full access key in the backend server,
you could create a simple drop that either has 3 keys or 1 key that is claimable 3 times.
In the drop, you'd specify that each time the key is claimed, the specified account would
receive 10 $NEAR.

#### Recurring Payments

Recurring payments are quite a common situation. If you need to send someone 10 $NEAR once a
month for 6 months, you could create a simple drop that has a usage config with an `interval` of 1 month.
In addition, you can set the time based config to have a `start` of  next week. Everytime the key is used, 
10 $NEAR is sent to the account. If the contractor missed a month's payment, they can claim the key late but 
can never use the key more than what is intended.


#### Quick Onboarding

If you need to quickly onboard users onto NEAR, you could create a simple drop with a
small amount of $NEAR (enough to create a wallet) and set the usage's permissions to be
`create_account_and_claim`. This means that the key can only be used to create accounts.
You can then add keys as you wish to the drop and give them out to users so they can create
accounts and be onboarded onto NEAR.

#### Lazy Registering Keys

A unique use-case for simple drops is the ability to lazy register key uses. This allows the funder to batch
create many keys at a time while only paying for basic fees such as the storage used and the key's allowance.
The funder would **not** need to pay for the `deposit_per_use` of each key up front. They can instead register individual
key uses as they are needed.

With this scenario, if an organization wanted to onboard users with a linkdrop valued at 10 $NEAR, they could create 1000 keys
without needing to pay 1000 * 10 = 10,000 $NEAR up-front. They could then register keys on an as-needed basis. If they need to
register 25 keys at a time, they can do this by simply calling the `register_uses` function.


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
drop has across all its keys. For basic simple drops that are *not* lazy registering keys, this field 
doesn't matter since all the uses are paid for up-front when the drop is created or when keys are added. 
With NFT drops, however, there is a 2 step process:
- Firstly, the drop is created and all the $NEAR required is pre-paid for. This is the same as
simple drops, however, the `registered_uses` are set to 0.
- Once the drop is created, the owner must send the contract the NFTs in order for keys to be
usable. This process is done through the `nft_transfer_call` workflow baked into the NFT standards.
It's up to the owner to facilitate this process.

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
}
```

By specifying this information, the drop is locked into only accepting NFTs from the sender and contract.

# Fungible Token Drops

A Fungible Token drop is also a light wrapper around the simple drop. It works very similarly to how its NFT
counterpart does. First, you'll need to create the drop and then you can fund it with assets and register
key uses.

You can preload a drop with as many FTs as you'd like even if you don't have the keys yet. This will spike the
`registered_uses` and then you can create keys and slowly eat away from this "total supply" overtime. If the
drop runs out, you can send it more FTs to top up. All the keys in the FT drop will share from this supply
and everytime a key is used, the `registered_uses` will decrement and the "total supply" will get smaller.

## How does it work?

As mentioned in the NFT section, every drop has a field known as `registered_uses`. This tells the contract
how many uses the drop has across all its keys. For basic simple drops that are *not* lazy registering keys, this field 
doesn't matter since all the uses are paid for up-front when the drop is created or when keys are added.
With FT drops, however, there is a 2 step process:
- Firstly, the drop is created and all the $NEAR required is pre-paid for. This is the same as
simple drops, however, the `registered_uses` are set to 0.
- Once the drop is created, the owner must send the contract the FTs in order for keys to be
usable. This process is done through the `ft_transfer_call` workflow baked into the FT standards.
It's up to the owner to facilitate this process.

## Use Cases

FT drops have some awesome flexibility due to the fact that they support all the functionalities of the Simple drops, just with
more use-cases and possibilities. Let's look at some use cases to see how fungible token drops can be used.

#### Recurring Payments

Recurring payments are quite a common situation. Let's say you need to send someone $50 USDC every week. You
could create a key with 5 uses that has a time config `interval` of 1 week. You would then pre-load maybe the
first week's deposit of $50 USDC and register 1 use or you could send $500 USDC for the first 10 weeks. At that
point, you would simply hand over the key to the user and they can claim once a week.

#### Backend Servers

Taking the recurring payments problem to another level, imagine that instead of leaving the claims up to the
contractor, you wanted to automatically pay them through a backend server. They would give you their NEAR account
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

You can pair this with setting the usage config's `refund_deposit` flag to true which would make it so that if anyone claims
the fungible tokens and they *already have a wallet*, it will automatically refund you the 0.02 $NEAR. That money should
only be used for the creation of new wallets. Since your focus is on the fungible tokens, you don't want to **force users**
to create a new wallet if they have one already by specifying the usage permissions to be `create_account_and_claim` but instead,
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

# Function Call Drops

Function call drops are by far the most powerful feature that Keypom provides. FC drops allow **any** method on **any**
contract to be executed (with some exceptions). In addition, there are a huge variety of customizations and features you can choose from when
defining the drop that come on top of the global options. The possibilities are almost endless. State of the art NFT ticketing,
lazy minting NFTs, auto registration into DAOs, analytics for marketing at events and much more.

## How does it work?

Unlike NFT and FT drops, the function calls must have everything paid for **upfront**. There is no two step process
so the creation is similar to Simple drops. Once the drop is created and keys are added, you can immediately start using it.

#### Function Call Config

When creating the drop, you have quite a lot of customization available. At the top level, there is a FC drop global
config similar to how the *general* config works.

```rust
pub struct FCConfig {
    /// How much GAS should be attached to the function call if it's a regular claim.
    /// If this is used, you *cannot* go through conventional linkdrop apps such as mynearwallet
    /// since those *always* attach 100 TGas no matter what. In addition, you will only be able to
    /// call `claim` if this is specified. You cannot have an `attached_gas` parameter and also
    /// call `create_account_and_claim.
    pub attached_gas: Option<Gas>,
}
```

#### Method Data

In addition to the global config, the user can specify a set of what's known as `MethodData`. This represents the
information for the function being called. Within this data, there are also a few optional configurations you can use
to extend your use cases. You'll see how powerful these can be in the use cases [section](#use-cases).

```rust
pub struct MethodData {
    /// Contract that will be called
    pub receiver_id: AccountId,
    /// Method to call on receiver_id contract
    pub method_name: String,
    /// Arguments to pass in (stringified JSON)
    pub args: String,
    /// Amount of yoctoNEAR to attach along with the call
    pub attached_deposit: U128,
    /// Specifies what field the claiming account should go in when calling the function
    /// If None, this isn't attached to the args
    pub account_id_field: Option<String>,
    /// Specifies what field the drop ID should go in when calling the function.
    /// If Some(String), attach drop ID to args. Else, don't attach.
    pub drop_id_field: Option<String>,
    /// Specifies what field the key ID should go in when calling the function.
    /// If Some(String), attach key ID to args. Else, don't attach.
    pub key_id_field: Option<String>,
}
```

The MethodData keeps track of the method being called, receiver, arguments, and attached deposit. In addition, there are
some optional fields that can be used to extend the use cases. If you have a contract that requires some more context from
Keypom such as the drop ID, key ID, or account ID that used the key, these can all be specified.

We've kept it generic such that you can specify the actual argument name that these will be passed in as. For example, if you
had a contract that would lazy mint an NFT and it required the account to be passed in as `receiver_id`, you could specify
an `account_id_field` set to `receiver_id` such that Keypom will automatically pass in the account ID that used the key under the
field `receiver_id`.

This logic extends to the drop ID, and key Id as well.

#### Key Uses

For **every key use**, you can specify a *vector* of `MethodData` which allows you to execute multiple function calls each
time a key is used. These calls are scheduled 1 by 1 using a simple for loop. This means that most of the time, the function
calls will be executed in the order specified in the vector but it is not *guaranteed*.

It's important to note that the Gas available is split evenly between *all* the function calls and if there are too many,
you might run into issues with not having enough Gas. You're responsible for ensuring that this doesn't happen.

The vector of `MethodData` is *optional* for each key use. If a key use has `null` rather than `Some(Vector<MethodData>)`,
it will decrement the uses and work as normal such that the `timestamp, `start` etc. are enforced. The only
difference is that after the key uses are decremented and these checks are performed, the execution **finishes early**. The null
case does **not** create an account or send *any* funds. It doesn't invoke any function calls and simply *returns once the
checks are done*. This makes the null case act as a "burner" where you disregard any logic. This has many uses which will
be explored in the use cases [section](#use-cases).

If a key has more than 1 use, you can specify a *different vector* of `MethodData` for **each use**. As an example, you could
specify that the first use will result in a null case and the second use will result in a lazy minting function being called.
If you have multiple uses but want them all to do the same thing, you don't have to repeat the same data. Passing in only 1
vector of `MethodData` will result in  **all the uses** inheriting that data.

## Security

Since all FC drops will be signed by the Keypom contract, there are a few restrictions in place to avoid malicious behaviors.
To avoid users from stealing registered assets from other drops, the following methods cannot be called via FC Drops:

```rust
/// Which methods are prohibited from being called by an FC drop
const DEFAULT_PROHIBITED_FC_METHODS: [&str; 6] = [
    "nft_transfer",
    "nft_transfer_call",
    "nft_approve",
    "nft_transfer_payout",
    "ft_transfer",
    "ft_transfer_call",
];
```

In addition, the Keypom contract cannot be the receiver of any function call. This is to avoid people
from calling private methods through FC Drops.

#### Keypom Arguments

When a key is used and a function is called, there is a data structure that is **automatically** attached to the arguments.
This is known as the `keypom_args`. It contains the information that the drop creator specified in the `MethodData`.

```rust
pub struct KeypomArgs {
    pub account_id_field: Option<String>,
    pub drop_id_field: Option<String>,
    pub key_id_field: Option<String>,
}
```

##### Motivation

Let's say there was an exclusive NFT contract that allowed the Keypom contract to mint NFTs as part of an FC drop. Only Keypom
was given access to mint the NFTs so they could be given out as linkdrops. The organizer only wanted links that were part of their
drop to be valid. For this reason, the NFT contract would only mint if Keypom called the `nft_mint` function and there was a field
`series` passed in and it was equal to the drop ID created by the organizer.

Let's say the owner created an exclusive drop that happened to have a drop ID of 5. They could then go to the NFT contract
and restrict NFTs to only be minted if:
- `series` had a value of 5.
- The Keypom contract was the one calling the function.

In order for this to work, when creating the drop, the owner would need to specify that the`drop_id_field` was set to a value of `series`
such that the drop ID is correctly passed into the function.

The problem with this approach is that the NFT contract has no way of knowing which arguments were sent by the **user** when the drop
was created `as part of the MethodData `args` and which arguments are automatically populated by the Keypom contract. There is nothing
stopping a malicious user from creating a new drop that has an ID of 6 but hardcoding in the actual arguments that `series` should have
a value of 5. In this case, the malicious drop would have *no* `drop_id_field` and the NFT contract would have no way of knowing that the
`series` value is malicious.

This can be prevented if a new field is introduced representing what was automatically injected by the Keypom contract itself. At the
end of the day, Keypom will **always** send correct information to the receiving contracts. If those contracts have a way to know what has
been sent by Keypom and what has been manually set by users, the problem is solved. In the above scenario, the NFT contract would simply add
an assertion that the `keypom_args` had the `account_id_field` set to `Some(series)` meaning that the incoming `series` field was set by Keypom
and not by a malicious user.

## Use Cases

Function call drops are the bread and butter of the Keypom contract. They are the most powerful and complex drops that can currently be created.
With this complexity, there are an almost infinite number of use-cases that arise.

#### Proof of Attendance Protocols

A very common use case in the space is what's known as Proof of Attendance. Often times when people go to events, they want a way to prove
that they were there. Some traditional approaches would be to submit your wallet address and you would be sent an NFT or some other form of
proof at a later date. The problem with this is that it has a very high barrier to entry. Not everyone has a wallet.

With Keypom, you can create a function call drop that allows people to onboard onto NEAR if they don't have a wallet or if they do, they can
simply use that. As part of the onboarding / claiming process, they would receive some sort of proof of attendance such as an NFT. This can
be lazy minted on-demand such that storage isn't paid up-front for all the tokens.

At this point, the event organizers or the funder can distribute links to people that attend the event in-person. These links would then be
claimed by users and they would receive the proof of attendance.

#### Auto Registration into DAOs

DAOs are a raging topic in crypto. The problem with DAOs, however, is there is a barrier to entry for users that aren't familiar with the
specific chain they're built on top of. Users might not have wallets or understand how to interact with contracts. On the contrary, they
might be very well versed or immersed in the DAO's topics. They shouldn't be required to create a wallet and learn the onboarding process.

With Keypom, you can create a function call drop with the main purpose of registering users into a DAO. For people that have a wallet,
this will act as an easy way of registering them with the click of a link. For users that don't have a wallet and are unfamiliar with
NEAR, they can be onboarded and registered into the DAO with the same click of a link.

#### Multisig Contracts

Another amazing use-case for Keypom is allowing multisig contracts to have ZERO barrier to entry. Often times when using a multisig contract,
you will entrust a key to a trusted party. This party might have no idea what NEAR is or how to interact with your contract. With Keypom,
you can create a drop that will allow them to sign their transaction with a click of a link. No NEAR wallet is needed and no knowledge of the
chain is required.

At the end of the day, from the users perspective, they are given a link and when they click it, their portion of the multisig transaction is
signed. The action is only performed on the multisig contract once all links have been clicked. This is an extremely powerful way of doing
accomplishing multisig transactions with zero barrier to entry.

The users don't even need to create a new account. They can simply call `claim` when the link is clicked which will fire the cross-contract call
to the multisig contract and pass in the keypom arguments that will be cross-checked by that contract.

#### NFT Ticketing

The problem with current NFT ticketing systems is that they require users to have a wallet. This is a huge barrier to entry for people that
are attending events but don't have wallets. In addition, there is often no proof of attendance for the event as the NFT is burned in order
to get into the event which requires an internet connection.

Keypom aims to solve these problems by having a ticketing system that has the following features.
- No wallet is needed to enter the event or receive a POAP.
- No wifi is needed at the door.
- An NFT is minted on-demand for each user that attends the event.
- Users can optionally onboard onto NEAR if they don't have a wallet.

In addition, some way to provide analytics to event organizers that contains information such as links that were:
- Given out but not clicked at all.
- Clicked but not attended.
- Partially claimed indicating the number of people that attended but did not onboard or receive a POAP.
- Fully claimed indicating the number of people that attended and received a POAP.

In order to accomplish this, you can create a drop that has 3 uses per key. These uses would be:
1. Array(`null`)
2. Array(`null`)
3. Array(function call to POAP contract to lazy mint an NFT)

The event organizer would create the links and distribute them to people however they see fit. When a user receives the link, the first
claim is automatically fired. This is a `null` case so nothing happens except for the fact that the key uses are decremented. At this point,
the organizer knows that the user has clicked the link since the uses have been decremented.

The next claim happens **only** when the user is at the door. Keypom would expose a QR code that can only be scanned by the bouncer's phone.
This QR code would appear once the first link is clicked and contains the private key for the link. At the event, they wouldn't need any wifi
to get in as they only need to show the bouncer the QR code. Once the bouncer scans it, the site would ensure that they have exactly 2 out of
the 3 uses left. If they don't, they're not let in. At that point, a use is decremented from the key and the next time they visit the
ticket page (when they have internet), they would be able to claim the final use and be onboarded / receive a POAP.

## Password Protected Keys

Password protecting key uses is an extremely powerful feature that can unlock many use-cases. Keypom has baked flexibility and customization
into the contract such that almost all use-cases involving password protection can be accomplished. Whenever a key is added to a drop, it can
have a unique password for each individual use, or it can one password for all uses in general.

### How Does It Work?

The Keypom implementation has been carefully designed so that users can't look at the NEAR Explorer to view what was passed into the contract
either when the drop was created or when a key was used to try and copy those passwords. We also want passwords to be unique across keys so that
if you know the password for 1 key, it doesn't work on a different key. In order to accomplish this, we use the concept of hashing.

Imagine you have a drop with 2 keys and you want to password protect each key. Rather than forcing the drop funder to input a unique password for 
each key and having them remember each one, we can have them input a single **base password** and derive unique passwords from it that are paired 
with the key's public key.

This is the most scalable option as it allows the drop funder to only need to remember 1 password and they can derive all the other ones using the
hashing algorithm and public key.

In the above scenario, let's say the funder inputs the base password as `mypassword1`. If a user wanted to claim the first key, they would need to input
into the contract:

`hash("mypassword1" + key1_public_key)`

The funder would need to give the user this hash somehow (such as embedding it into the link or having an app that can derive it). It's important to note 
that the funder should probably **NOT** give them the base password otherwise the user could derive the passwords for all other keys (assuming those keys have 
the same base password).

### What is Stored On-Chain?

How does Keypom verify that the user passed in the correct password? If the funder were to simply pass in `hash("mypassword1" + key1_public_key)` into the
contract as an argument when the key is created, users could just look at the NEAR Explorer and copy that value. 

Instead, the funder needs to pass in a double hash when the key is created: `hash(hash("mypassword1" + key1_public_key))`. 

This is the value that is stored on-chain and when the user tries to claim the key, they would pass in just the single hash: `hash("mypassword1" + key1_public_key)`.  
The contract would then compute `hash(hash("mypassword1" + key1_public_key))` and compare it to the value stored on-chain. If they match, the key is claimed.

Using this method, the base password is not exposed to the user, nobody can look on-chain or at the NEAR explorer and derive the password, and the password is unique
across multiple keys.

## Passwords Per Key Use

Unlike the passwords per key which is the same for all uses of a key, the drop creator can specify a password for each individual key use. This password follows
the same pattern as the passwords per key in that the funder inputs a `hash(hash(SOMETHING))` and then the user would input `hash(SOMETHING)` and the contract
would hash this and compare it to the value stored on-chain.

The difference is that each individual key use can have a different value stored on-chain such that the user can be forced to input a different hash each time.
This `SOMETHING` that is hashed can be similar to the global password per key example but this time, the desired key use is added: `hash("mypassword1" + key1_public_key + use_number)`

In order to pass in the passwords per use, a new data structure is introduced so you only need to pass in passwords for the uses that have them. This is known as the 
`JsonPasswordForUse` and is as follows:

```rust
pub struct JsonPasswordForUse {
    /// What is the password for this use (such as `hash("mypassword1" + key1_public_key + use_number)`)
    pub pw: String,
    /// Which use does this pertain to
    pub key_use: u64
}
````

## Adding Your First Password

Whenever keys are added to Keypom, if there's passwords involved, they must be passed in using the following format. 

```rust
passwords_per_use: Option<Vec<Option<Vec<JsonPasswordForUse>>>>,
passwords_per_key: Option<Vec<Option<String>>>,
```

Each key that is being added either has a password, or doesn't. This is through the `Vec<Option<>`. This vector **MUST** be the same length as the number of keys created.This doesn't 
mean that every key needs a password, but the Vector must be the same length as the keys.

As an example, if you wanted to add 3 keys to a drop and wanted only the first and last key to have a password_per_key, you would pass in:
```rust
passwords_per_key: Some(vec![Some(hash(hash(STUFF))), None, Some(hash(hash(STUFF2)))])
```

## Complex Example

To help solidify the concept of password protected keys, let's go through a complex example. Imagine Alice created a drop with a `uses_per_key` of 3.
She wants to create 4 keys: 
- Key A: No password protection.
- Key B: Password for uses 1 and 2.
- Key C: Password for use 1 only.
- Key D: Password that doesn't depend on the use.

In this case, for Keys B and C, they will have the same base password but Alice wants to switch things up and have a different base password for Key D.
When these keys are added on-chain, the `passwords_per_key` will be passed in as such:

```rust
passwords_per_key: Some(vec![
    None, // Key A
    None, // Key B
    None, // Key C
    // Key D
    Some(
        hash(hash("key_d_base_password" + key_d_public_key))
    ), 
]),
```
The passwords for Key B and Key C will be passed in as such:

```rust
passwords_per_use: Some(vec![
    None, // Key A

    // Key B
    vec![
        {
            pw: hash(hash("keys_bc_base_password" + key_b_public_key + "1")),
            key_use: 1
        },
        {
            pw: hash(hash("keys_bc_base_password" + key_b_public_key + "2")),
            key_use: 2
        }
    ]

    // Key C
    vec![
        {
            pw: hash(hash("keys_bc_base_password" + key_c_public_key + "1")),
            key_use: 1
        }
    ]

    None // Key D
]),
```

The drop funder would then give the keys out to people:

### Key A
Alice gives Bob Key A and he would be able to claim it 3 times with no password required.

### Key D
Alice gives Charlie Key D and he would be able to claim it 3 times with the hashed global key password: `hash("key_d_base_password" + key_d_public_key)`.
When Charlie uses the key, he would input the password `hash("key_d_base_password" + key_d_public_key)` and the contract would hash that and check to see
if it matches what is stored on-chain (which it does).

If anyone tried to look at what Charlie passes in through the explorer, it wouldn't work since his hash contains the public key for key D and as such it is only
valid for Key D.

Similarly, if Charlie tried to look at the explorer when Alice created the keys and attempted to pass in `hash(hash("key_d_base_password" + key_d_public_key))`, 
the contract would attempt to hash this and it would NOT match up with what's in the storage.

### Key B
Alice gives Eve Key B and she would need a password for claim 1 and 2. For the first claim, she needs to pass in: `hash("keys_bc_base_password" + key_b_public_key + "1")`.
The contract would then check and see if the hashed version of this matches up with what's stored on-chain for that use.

The second time Eve uses the key, she needs to pass in hash("keys_bc_base_password" + key_b_public_key + "2") and the same check is done.

If Eve tries to pass in `hash("keys_bc_base_password" + key_b_public_key + "1")` for the second key use, the contract would hash it and check:

hash(hash("keys_bc_base_password" + key_b_public_key + "1")) == hash(hash("keys_bc_base_password" + key_b_public_key + "2"))

Which is incorrect and the key would not be claimed.

Once Eve uses the key 2 times, the last claim is not password protected and she's free to claim it.

Key C is similar to Key B except that it only has 1 password for the first use.

## Use-Cases

Password protecting key uses is a true game changer for a lot of use-cases spanning from ticketing to simple marketing and engagement.

#### Ticketing and POAPs

Imagine you had an event and wanted to give out exclusive POAPs to people that came. You didn't want to force users to: 
- Have a NEAR wallet
- Have wifi at the door.
- Burn NFTs or tokens to get into the event.

The important thing to note is that by using password protected key uses, you can **GUARANTEE** that anyone that received a POAP had to
**PHYSICALLY** show up to the event. This is because the POAP would be guarded by a password.

You could create a ticketing event using Keypom as outlined in the [Ticketing](#nft-ticketing) section and have a key with 2 uses. The first use 
would be password protected and the second use is not. The first use will get you through the door and into the event and the second
contains the exclusive POAP and can onboard you. This means that anyone with the ticket, or key, can only receive the POAP if they know the password.

You can have a scanner app that would scan people's tickets (tickets are just the private key). In this scanner app, the *base password* is stored and 
whenever the ticket is scanned, the public key is taken and the following hash is created:

`hash(base password + public key)`

This hash is then used to claim a use of the key and you will be let into the party. The scanner app can deterministically generate all the
necessary hashes for all the tickets by simply scanning the QR code (which has the private key exposed). The tickets are worthless unless
you actually show up to the event and are scanned.

Once you're scanned, you can refresh your ticket page and the use the second key claim which is not password protected. This use contains the
exclusive POAP and you can onboard onto NEAR.

#### Marketing and Engagement

Let's say that you're at an event and want people to show up to your talks and learn about your project. You can have a scanner app similar to the
one mentioned in the ticketing scenario that derives the password for any use on any key.

At the beginning of the event, you can give out a bunch of keys that have progressively increasing rewards gated by a password. At the end, the last
key use contains a special reward that is only unlocked if the user has claimed all the previous key uses.

In order for these uses to be unlocked, People must show up to your talks and get scanned. The scanner will derive the necessary password and unlock 
the rewards. Users will only get the exclusive reward if they come to ALL your talks.

This idea can be further expanded outside the physical realm to boost engagement on your websites as an example:

You want users to interact with new features of your site or join your mailing list.

You can have links where uses are ONLY unlocked if the user interacts with special parts of your site such as buying a new NFT or joining your mailing list 
or clicking an easter egg button on your site etc.
*/

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, LookupSet, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::json;
use near_sdk::{
    env, ext_contract, near_bindgen, promise_result_as_success, require, AccountId, Balance,
    BorshStorageKey, CryptoHash, Gas, PanicOnDefault, Promise, PromiseOrValue, PromiseResult,
    PublicKey,
};

/// The minimum amount of storage required to create an access key on the contract
/// Value equates to 0.001 $NEAR per key.
const ACCESS_KEY_STORAGE: u128 = 1_000_000_000_000_000_000_000;

/// The minimum amount of NEAR that it costs to create a new *named* account on NEAR.
/// This is based off the longest possible account ID length and has a value of 0.00284 $NEAR
const NEW_ACCOUNT_BASE: u128 = 2_840_000_000_000_000_000_000;

/// Constant indicating no attached deposit should be sent to a function call.
/// Declared for readability and to prevent magic numbers.
const NO_DEPOSIT: u128 = 0;

/// Minimum Gas required for the on claim callback.
/// This value equates to 55 TGas
const MIN_GAS_FOR_ON_CLAIM: Gas = Gas(55_000_000_000_000);

/// Minimum Gas required for a simple NFT transfer
/// This value equates to 10 TGas
const MIN_GAS_FOR_SIMPLE_NFT_TRANSFER: Gas = Gas(10_000_000_000_000);

/// Minimum Gas required to resolve the NFT transfer
/// This is 15 TGas more than the simple NFT transfer.
/// The total value is 15 TGas + 10 TGas = 25 TGas
const MIN_GAS_FOR_RESOLVE_TRANSFER: Gas =
    Gas(15_000_000_000_000 + MIN_GAS_FOR_SIMPLE_NFT_TRANSFER.0);

/// Actual amount of GAS to attach when querying the storage balance bounds.
/// No unspent GAS will be attached on top of this (weight of 0)
const GAS_FOR_STORAGE_BALANCE_BOUNDS: Gas = Gas(10_000_000_000_000);
/// Minimum Gas required to resolve the cross contract call to the FT contract checking for storage balances.
/// This value equates to 150 TGas
const MIN_GAS_FOR_RESOLVE_STORAGE_CHECK: Gas = Gas(150_000_000_000_000);
/// Minimum Gas required to perform a simple transfer of fungible tokens.
/// This value equates to 5 TGas
const MIN_GAS_FOR_FT_TRANSFER: Gas = Gas(5_000_000_000_000);
/// Minimum Gas required to register a user on the FT contract
/// This value equates to 5 TGas
const MIN_GAS_FOR_STORAGE_DEPOSIT: Gas = Gas(5_000_000_000_000);
/// Minimum Gas required to resolve the batch of promises for transferring the FTs and registering the user.
/// This value is made up of the Gas for transferring, Gas for registering, and a 13 TGas buffer
/// Equal to 13 TGas + 5 TGas + 5 TGas = 23 TGas
const MIN_GAS_FOR_RESOLVE_BATCH: Gas =
    Gas(13_000_000_000_000 + MIN_GAS_FOR_FT_TRANSFER.0 + MIN_GAS_FOR_STORAGE_DEPOSIT.0);

/// Specifies the GAS being attached from the wallet site
/// If no specific Gas value is specified and overloaded, this value is used.
/// Value equates to 100 TGas
const ATTACHED_GAS_FROM_WALLET: Gas = Gas(100_000_000_000_000);

/// Specifies the amount of GAS to attach on top of the FC Gas if executing a regular function call in claim
/// This is to to ensure there is enough Gas to execute everything except the CCC
/// This value is equal to 20 TGas
const GAS_OFFSET_IF_FC_EXECUTE: Gas = Gas(20_000_000_000_000);

/// Actual amount of GAS to attach for creating a new account.
/// This value is equal to 28 TGas
const GAS_FOR_CREATE_ACCOUNT: Gas = Gas(28_000_000_000_000);

/// Specifies both `claim` and `create_account_and_claim` functions can be called with the access key
const ACCESS_KEY_BOTH_METHOD_NAMES: &str = "claim,create_account_and_claim";

/// Specifies only `claim` can be called with the access key
const ACCESS_KEY_CLAIM_METHOD_NAME: &str = "claim";

/// Specifies only `create_account_and_claim` can be called with the access key
const ACCESS_KEY_CREATE_ACCOUNT_METHOD_NAME: &str = "create_account_and_claim";

/// Fee for creating a drop. Currently 0 $NEAR
const DROP_CREATION_FEE: u128 = 0;
/// Fee for adding a key. Currently 0 $NEAR
const KEY_ADDITION_FEE: u128 = 0;

/// How much to decrement the access key's allowance if there is a soft panic
/// Value is equal to 10 TGas
const GAS_FOR_PANIC_OFFSET: Gas = Gas(10_000_000_000_000);

/// Which methods are prohibited from being called by an FC drop
const DEFAULT_PROHIBITED_FC_METHODS: [&str; 6] = [
    "nft_transfer",
    "nft_transfer_call",
    "nft_approve",
    "nft_transfer_payout",
    "ft_transfer",
    "ft_transfer_call",
];

mod internals;
mod models;
mod stage1;
mod stage2;
mod stage3;
mod views;

use internals::*;
use models::*;
use stage2::*;

/// Contract metadata structure
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractSourceMetadata {
    /// Commit hash being used for the currently deployed wasm. If the contract is not open-sourced, this could also be a numbering system for internal organization / tracking such as "1.0.0" and "2.1.0".
    pub version: String,
    /// Link to open source code such as a Github repository or a CID to somewhere on IPFS.
    pub link: String,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    DropIdForPk,
    DropsForId,
    DropIdsForFunder,
    DropIdsForFunderInner { account_id_hash: CryptoHash },
    PksForDrop { account_id_hash: CryptoHash },
    PasswordsPerUse { account_id_hash: CryptoHash },
    DropMetadata { account_id_hash: CryptoHash },
    TokenIdsForDrop { account_id_hash: CryptoHash },
    FeesPerUser,
    UserBalances,
    ProhibitedMethods,
    RegisteredFtContracts,
    ContractMetadata,
}

#[near_bindgen]
#[derive(PanicOnDefault, BorshDeserialize, BorshSerialize)]
/// Main contract struct that holds all the contract data
pub struct Keypom {
    /// Owner of the Keypom contract that can call internal methods (e.g. `set_fees`)
    pub owner_id: AccountId,
    /// Which contract is the actual linkdrop deployed to (i.e `testnet` or `near`)
    pub root_account: AccountId,

    /// Map of each key to its respective drop ID. This is much more efficient than repeating the
    /// Drop data for every single key.
    pub drop_id_for_pk: UnorderedMap<PublicKey, DropId>,
    /// Map a drop ID to its respective Drop data
    pub drop_for_id: LookupMap<DropId, Drop>,
    /// Keep track of the drop ids that each funder has created. This is used for view methods.
    pub drop_ids_for_owner: LookupMap<AccountId, UnorderedSet<DropId>>,

    /// Fee taken by the contract every time a new drop is created
    pub drop_fee: u128,
    /// Fee taken by the contract everytime a new key is added to a drop
    pub key_fee: u128,
    /// Total amount of fees available for withdrawal collected overtime.
    pub fees_collected: u128,

    /// Overload the `drop_fee` and `key_fee` for specific users by providing custom fees
    /// Tuple is (drop_fee, key_fee)
    pub fees_per_user: LookupMap<AccountId, (u128, u128)>,

    /// Keep track of the balances for each user. This is to prepay for drop creations
    pub user_balances: LookupMap<AccountId, Balance>,

    /// Keep track of a nonce used for the drop IDs
    pub next_drop_id: DropId,

    /// How many yoctoNEAR does 1 unit of Gas cost
    pub yocto_per_gas: u128,

    /// Which methods are prohibited from being called with an access key through an FC Drop
    pub prohibited_fc_methods: LookupSet<String>,

    /// Which contract has Keypom been automatically registered on?
    pub registered_ft_contracts: LookupSet<AccountId>,

    /// Source metadata extension:
    pub contract_metadata: LazyOption<ContractSourceMetadata>,
}

#[near_bindgen]
impl Keypom {
    /// Initialize contract and pass in the desired deployed linkdrop contract (i.e testnet or near)
    #[init]
    pub fn new(
        root_account: AccountId,
        owner_id: AccountId,
        contract_metadata: ContractSourceMetadata,
    ) -> Self {
        let mut keypom = Self {
            owner_id,
            root_account,
            drop_id_for_pk: UnorderedMap::new(StorageKey::DropIdForPk),
            drop_for_id: LookupMap::new(StorageKey::DropsForId),
            drop_ids_for_owner: LookupMap::new(StorageKey::DropIdsForFunder),
            user_balances: LookupMap::new(StorageKey::UserBalances),
            next_drop_id: 0,
            prohibited_fc_methods: LookupSet::new(StorageKey::ProhibitedMethods),
            registered_ft_contracts: LookupSet::new(StorageKey::RegisteredFtContracts),
            /*
                FEES
            */
            fees_per_user: LookupMap::new(StorageKey::FeesPerUser),
            drop_fee: DROP_CREATION_FEE,
            key_fee: KEY_ADDITION_FEE,
            fees_collected: 0,
            yocto_per_gas: 100_000_000,

            /*
                CONTRACT METADATA
            */
            contract_metadata: LazyOption::new(
                StorageKey::ContractMetadata,
                Some(&contract_metadata),
            ),
        };

        // Loop through and add all the default prohibited methods to the set
        for method in DEFAULT_PROHIBITED_FC_METHODS {
            keypom.prohibited_fc_methods.insert(&method.to_string());
        }

        keypom
    }
}
