use crate::*;

// ------------------------ General ------------------------ //
/// How much yoctoNEAR is in 1 unit of Gas
pub const YOCTO_PER_GAS: Balance = 100_000_000;
/// For every cross contract call, there is a base 5 TGas cost on top of what is sent to the contract
pub const MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP: Gas = Gas(5_000_000_000_000); // 5 TGas
/// Maximum amount of Gas that can be attached to the transaction
pub const MAX_GAS_ATTACHABLE: Gas = Gas(300_000_000_000_000); // 300 TGas
/// Minimum amount of gas required to perform any necessary computations for the receipt
pub const MIN_GAS_FOR_RECEIPT_COMPUTATION: Gas = Gas(5_000_000_000_000); // 7.5 TGas

// ------------------------ Create Account & Claim ------------------------ //
/// The base amount of gas required for create_account_and_claim calls.
/// This does not include the actual asset promises
pub const BASE_GAS_FOR_CREATE_ACC_AND_CLAIM: Gas = Gas(BASE_GAS_FOR_CLAIM.0 + GAS_FOR_CREATE_ACCOUNT.0 + MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP.0);
/// Actual amount of GAS to attach for creating a new account.
/// This value is equal to 28 TGas
pub const GAS_FOR_CREATE_ACCOUNT: Gas = Gas(28_000_000_000_000); 
/// The base amount of gas required for the callback after a new account is created.
/// This does not include the actual asset promises
pub const BASE_GAS_FOR_RESOLVE_ACCOUNT_CREATION: Gas = Gas(MIN_GAS_FOR_RECEIPT_COMPUTATION.0 + MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP.0 + MIN_GAS_FOR_RESOLVE_ASSET_CLAIM.0);


// ------------------------ Claim ------------------------ //
/// The base amount of gas required for claim calls.
/// This does not include the actual asset promises
pub const BASE_GAS_FOR_CLAIM: Gas = Gas(BASE_GAS_FOR_RESOLVE_ACCOUNT_CREATION.0 + 2*MIN_GAS_FOR_RECEIPT_COMPUTATION.0);


// ------------------------ Shared Constants ------------------------ //
/// The minimum amount of gas required for the callback after a all the assets have been claimed.
pub const MIN_GAS_FOR_RESOLVE_ASSET_CLAIM: Gas = Gas(MIN_GAS_FOR_RECEIPT_COMPUTATION.0 + MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP.0);


// ------------------------ Pessimistic Allowance ------------------------ //
/// For pessimistic allowance calculations:
/// Lowest amount of Gas a CCC can use 
pub const GAS_PER_CCC: Gas = Gas(5_000_000_000_000); // 5 TGas
/// How much Gas it costs to spin up the receipt
pub const RECEIPT_GAS_COST: Gas = Gas(2_500_000_000_000); // 2.5 TGas


// ------------------------ Assets ------------------------ //
pub const GAS_FOR_NONE_ASSET: Gas = Gas(550_000_000_000); // 0.4 TGas
pub const GAS_FOR_NEAR_TRANSFER: Gas = Gas(9_000_000_000_000); // 9 TGas