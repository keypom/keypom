# Linkdrop Proxy Implementation

Why? Regular linkdrops are too expensive.

Creates linkdrops with the minimum amount required for an access key and then proxy calls the 'near' or 'testnet' accounts.

## Instructions

`yarn && yarn test:deploy`

#### Pre-reqs

Rust, cargo, near-cli, etc...
Everything should work if you have NEAR development env for Rust contracts set up.

[Tests](test/api.test.js)
[Contract](contract/src/lib.rs)