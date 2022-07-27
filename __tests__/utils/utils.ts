import { BN, NEAR } from "near-workspaces";

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

export function assertBalanceChange(b1: NEAR, b2: NEAR, expected: NEAR, precision: number) {
  // 1 * 5% = .05
  let divNum = new BN(Math.ceil(1 / precision))
  let range = expected.abs().div(divNum);
  let acceptableRange = {
    upper: expected.abs().add(range), // 1 + .05 = 1.05
    lower: expected.abs().sub(range) // 1 - .05  = .95
  }
  let diff = b2.sub(b1).abs();
  //console.log(`diff: ${diff.toString()} range: ${JSON.stringify(acceptableRange)}`)
  return diff.gte(acceptableRange.lower) && diff.lte(acceptableRange.upper)
}