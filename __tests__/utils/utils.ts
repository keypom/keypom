import { BN, NEAR, NearAccount } from "near-workspaces";
import { JsonDrop, JsonKeyInfo } from "./types";

export const DEFAULT_GAS: string = "30000000000000";
export const DEFAULT_DEPOSIT: string = "1000000000000000000000000";

export function defaultCallOptions(
  gas: string = DEFAULT_GAS,
  attached_deposit: string = DEFAULT_DEPOSIT
) {
  return {
    gas: new BN(gas),
    attachedDeposit: new BN(attached_deposit),
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

export async function queryAllViewFunctions(
  {
  contract,
  drop_id = null,
  key = null,
  from_index = '0',
  limit = 50,
  account_id = null
  }: 
  {
    contract: NearAccount,
    drop_id?: number | null,
    key?: string | null,
    from_index?: string | null,
    limit?: number | null,
    account_id?: string | null
  }
) {
  let getKeyBalance: string | null = null;
  let getKeyInformation: JsonKeyInfo | null = null;
  if(key != null) {
    getKeyBalance = await contract.view('get_key_balance', {key});
    getKeyInformation = await contract.view('get_key_information', {key});
  }

  let getDropInformation: JsonDrop | null = null;
  let getKeySupplyForDrop: number | null = null;
  let getKeysForDrop: JsonKeyInfo[] | null = null;
  let tokenIdsForDrop: string[] | null = null;
  if(drop_id != null) {
    getDropInformation = await contract.view('get_drop_information', {drop_id});
    getKeySupplyForDrop = await contract.view('get_key_supply_for_drop', {drop_id});
    getKeysForDrop = await contract.view('get_keys_for_drop', {drop_id, from_index, limit});
    tokenIdsForDrop = await contract.view('get_token_ids_for_drop', {drop_id, from_index, limit});
  }

  let keySupplyForOwner: number | null = null;
  let dropSupplyForOwner: number | null = null;
  let dropsForOwner: JsonDrop[] | null = null;
  if(account_id != null) {
    keySupplyForOwner = await contract.view('get_key_supply_for_owner', {account_id});
    dropSupplyForOwner = await contract.view('get_drop_supply_for_owner', {account_id});
    dropsForOwner = await contract.view('get_drops_for_owner', {account_id, from_index, limit});
  }

  let getGasPrice: string = await contract.view('get_gas_price', {});
  let getRootAccount: string = await contract.view('get_root_account', {});
  let getFeesCollected: string = await contract.view('get_fees_collected', {});
  let getNextDropId: number = await contract.view('get_next_drop_id', {});
  let keyTotalSupply: string = await contract.view('get_key_total_supply', {});
  let getKeys: JsonKeyInfo[] = await contract.view('get_keys', {from_index, limit});

  return {
    keyBalance: getKeyBalance,
    keyInformation: getKeyInformation,
    dropInformation: getDropInformation,
    keySupplyForDrop: getKeySupplyForDrop,
    keysForDrop: getKeysForDrop,
    tokenIdsForDrop: tokenIdsForDrop,
    keySupplyForOwner: keySupplyForOwner,
    dropSupplyForOwner: dropSupplyForOwner,
    dropsForOwner: dropsForOwner,
    gasPrice: getGasPrice,
    rootAccount: getRootAccount,
    feesCollected: getFeesCollected,
    nextDropId: getNextDropId,
    keyTotalSupply: keyTotalSupply,
    keys: getKeys,
  }
}