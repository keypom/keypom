import { BN } from "near-workspaces";

export const DEFAULT_GAS: string = "30000000000000";
export const DEFAULT_DEPOSIT: string = "1000000000000000000000000";

export function defaultCallOptions(
  gas: string = DEFAULT_GAS,
  deposit: string = DEFAULT_DEPOSIT
) {
  return {
    gas: new BN(gas),
    attachedDeposit: new BN(deposit),
  };
}