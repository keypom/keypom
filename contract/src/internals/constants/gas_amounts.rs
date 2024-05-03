use crate::*;

// ------------------------ General ------------------------ //
/// For every cross contract call, there is a base 5 TGas cost on top of what is sent to the contract
pub const MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP: Gas = Gas::from_tgas(5); // 5 TGas
/// Maximum amount of Gas that can be attached to the transaction
pub const MAX_GAS_ATTACHABLE: Gas = Gas::from_tgas(300); // 300 TGas
/// Minimum amount of gas required to perform any necessary computations for the receipt
pub const MIN_GAS_FOR_RECEIPT_COMPUTATION: Gas = Gas::from_tgas(5); // 7.5 TGas

// ------------------------ Create Account & Claim ------------------------ //
/// The base amount of gas required for create_account_and_claim calls.
/// This does not include the actual asset promises
pub const BASE_GAS_FOR_CREATE_ACC_AND_CLAIM: Gas = Gas::from_gas(
    BASE_GAS_FOR_CLAIM.as_gas()
        + GAS_FOR_CREATE_ACCOUNT.as_gas()
        + MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP.as_gas(),
);
/// Actual amount of GAS to attach for creating a new account.
/// This value is equal to 28 TGas
pub const GAS_FOR_CREATE_ACCOUNT: Gas = Gas::from_tgas(28);
/// The base amount of gas required for the callback after a new account is created.
/// This does not include the actual asset promises
pub const BASE_GAS_FOR_RESOLVE_ACCOUNT_CREATION: Gas = Gas::from_gas(
    MIN_GAS_FOR_RECEIPT_COMPUTATION.as_gas()
        + MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP.as_gas()
        + MIN_GAS_FOR_RESOLVE_ASSET_CLAIM.as_gas(),
);

// ------------------------ Claim ------------------------ //
/// The base amount of gas required for claim calls.
/// This does not include the actual asset promises
pub const BASE_GAS_FOR_CLAIM: Gas = Gas::from_gas(
    BASE_GAS_FOR_RESOLVE_ACCOUNT_CREATION.as_gas() + 2 * MIN_GAS_FOR_RECEIPT_COMPUTATION.as_gas(),
);

// ------------------------ Shared Constants ------------------------ //
/// The minimum amount of gas required for the callback after a all the assets have been claimed.
pub const MIN_GAS_FOR_RESOLVE_ASSET_CLAIM: Gas = Gas::from_gas(
    MIN_GAS_FOR_RECEIPT_COMPUTATION.as_gas() + MIN_BASE_GAS_FOR_RECEIPT_SPIN_UP.as_gas(),
);

// ------------------------ Assets ------------------------ //
pub const GAS_FOR_NONE_ASSET: Gas = Gas::from_gas(400_000_000_000); // 0.4 TGas
pub const GAS_FOR_NEAR_TRANSFER: Gas = Gas::from_tgas(9); // 9 TGas
